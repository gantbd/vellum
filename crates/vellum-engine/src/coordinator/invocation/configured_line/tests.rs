use super::*;
use crate::settings::*;
fn identity() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "host".into(),
        incarnation: 1,
    }
}
fn setup() -> (
    SourceCoordinator,
    LogicalViewContext,
    SettingsTargetId,
    Recorder,
) {
    let (mut o, c, mut r) = views::line_test_fixture();
    o.invocations = InvocationTable::new(InvocationLimits {
        max_lifetime_issued: 16,
        max_retained: 16,
        max_intent_bytes: 16384,
    });
    let mut d = SettingsDefinitionV1 {
        schemas: vec![
            SettingSchemaV1 {
                key: "unit".into(),
                value_type: SettingTypeV1::Text,
                default: SettingValueV1::Text("  ".into()),
                domain: SettingDomainV1::Source,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::Replace,
                sensitive: false,
            },
            SettingSchemaV1 {
                key: "width".into(),
                value_type: SettingTypeV1::Integer { min: 1, max: 16 },
                default: SettingValueV1::Number(4.0),
                domain: SettingDomainV1::Source,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::Replace,
                sensitive: false,
            },
        ],
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
    };
    let alternate = SettingIdentityV1 {
        id: "alternate".into(),
        ..identity()
    };
    d.layers.push(SettingLayerV1 {
        identity: alternate,
        scope: SettingScopeV1::Host,
        generation: 1,
        values: vec![
            (
                "unit".into(),
                SettingContributionV1::Set(SettingValueV1::Text("\t".into())),
            ),
            (
                "width".into(),
                SettingContributionV1::Set(SettingValueV1::Number(8.0)),
            ),
        ],
    });
    d.targets.push(SettingTargetV1 {
        identity: SettingIdentityV1 {
            id: "peer".into(),
            ..identity()
        },
        layers: vec![SettingIdentityV1 {
            id: "alternate".into(),
            ..identity()
        }],
    });
    o.install_initial_settings(
        compile_settings_v1(
            &d,
            "engine",
            SettingsLimitsV1 {
                max_keys: 8,
                max_layers: 8,
                max_targets: 8,
                max_identifier_bytes: 64,
                max_schema_nodes: 64,
                max_schema_depth: 32,
                max_value_nodes: 128,
                max_value_depth: 32,
                max_state_bytes: 65536,
                max_work: 65536,
            },
            &mut r,
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    let p = SettingsRetentionLimitsV1 {
        max_issued: 16,
        max_retained: 16,
        max_snapshot_bytes: 65536,
        max_pool_bytes: 2_097_152,
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
    let target = o.settings_target(&identity(), &mut r).unwrap();
    (o, c, target, r)
}
fn request(target: &SettingsTargetId) -> ConfiguredLineEditV1 {
    ConfiguredLineEditV1::Indent {
        target: target.clone(),
        unit_key: "unit".into(),
        tab_width_key: "width".into(),
    }
}
#[test]
fn configured_line_resolves_current_and_retained_targets_independently_after_storage_reordering() {
    let (mut o, c, target, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_configured_line_edit(&id, &c, &request(&target), &mut r)
        .unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 8,
            max_request_nodes: 64,
            max_request_depth: 32,
            max_request_bytes: 16384,
            max_plan_bytes: 262144,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    let p = o
        .prepare_settings_change(
            &[SettingChangeV1 {
                layer: SettingIdentityV1 {
                    id: "alternate".into(),
                    ..identity()
                },
                expected_generation: 1,
                key: "unit".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Text("   ".into())),
            }],
            &mut r,
        )
        .unwrap();
    o.commit_settings_change(p, &mut r).unwrap();
    let mut current = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
    current.reverse_targets_for_test();
    o.settings = Some(Arc::new(current));
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::Committed(_))
    ));
    assert_eq!(
        o.read(&c.source.document, &mut r).unwrap().source.text(),
        "   \tfoo"
    );
    assert_eq!(o.settings_target(&identity(), &mut r).unwrap(), target);
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().retained, 0);
}
#[test]
fn outer_and_inner_intent_thresholds_are_checked_before_charged_interpretation_issuance() {
    let (mut o, c, target, mut r) = setup();
    let req = request(&target);
    let outer = std::mem::size_of::<ConfiguredLineIntent>() + req.payload_bytes().unwrap();
    let inner = std::mem::size_of::<LineEditV1>()
        + std::mem::size_of::<LogicalViewContext>()
        + std::mem::size_of::<LineEditBudget>()
        + 2;
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = outer - 1;
    assert_eq!(
        o.submit_configured_line_edit(&id, &c, &req, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.invocations.limits.max_intent_bytes = outer + inner - 1;
    o.submit_configured_line_edit(&id, &c, &req, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::ConfiguredLineRejected(
            ConfiguredLineErrorV1::Limit
        ))
    );
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = outer + inner;
    o.submit_configured_line_edit(&id, &c, &req, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::Committed(_))
    ));
}
#[test]
fn no_op_configured_outdent_uses_no_exhausted_publication_or_history_counter() {
    let (mut o, c, target, mut r) = setup();
    o.set_logical_selections(
        &c,
        &[crate::selection::Selection {
            anchor: crate::coordinates::BytePosition(5),
            head: crate::coordinates::BytePosition(5),
            affinity: crate::attachment::Affinity::After,
        }],
        0,
        &mut r,
    )
    .unwrap();
    // The selected line has indentation; use an unindented replacement source first.
    let source = o.read(&c.source.document, &mut r).unwrap().version;
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: source,
                edits: vec![Replacement {
                    start: 0,
                    end: 2,
                    expected: " \t".into(),
                    inserted: String::new(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let c = o.logical_view(&c.view, &mut r).unwrap().context().unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let req = ConfiguredLineEditV1::Outdent {
        target,
        unit_key: "unit".into(),
        tab_width_key: "width".into(),
    };
    o.submit_configured_line_edit(&id, &c, &req, &mut r)
        .unwrap();
    let groups = o.history_usage().retained_groups;
    o.generation = u64::MAX;
    o.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = o.invocation_outcome(&id, &mut r).unwrap() else {
        panic!("no-op")
    };
    assert!(outcome.operation.is_none());
    assert_eq!(o.generation(), u64::MAX);
    assert_eq!(o.history_usage().retained_groups, groups);
}
#[test]
fn configured_capture_and_other_valid_line_preparations_cannot_be_substituted() {
    for drift in 0..3 {
        let (mut o, c, target, mut r) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        let req = request(&target);
        o.submit_configured_line_edit(&id, &c, &req, &mut r)
            .unwrap();
        let extra = if drift == 2 {
            Some(
                o.prepare_configured_line(
                    &c,
                    request(&target),
                    o.configured_line_limits.unwrap(),
                    16384,
                    &mut r,
                )
                .unwrap(),
            )
        } else {
            None
        };
        let extra_lease = extra.as_ref().map(|(_, w, _)| w.lease.clone());
        let slot = o.invocations.slots[id.slot].as_mut().unwrap();
        let Phase::ConfiguredLinePending { prepared, .. } = &mut slot.phase else {
            unreachable!()
        };
        if let Some((plan, _, _)) = extra {
            *prepared = Ok(plan)
        } else {
            let plan = prepared.as_mut().unwrap();
            if drift == 0 {
                plan.configured_line = None;
            } else {
                plan.configured_required = false;
            }
        }
        let generation = o.generation();
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
        o.release_configured_line_retention(&id, &mut r).unwrap();
        if let Some(lease) = extra_lease {
            o.release_settings_interpretation(&lease, &mut r).unwrap();
        }
        assert_eq!(o.settings_retention_usage().retained, 0);
    }
}
#[test]
fn changed_runtime_profiles_are_known_stale_not_refreshed_or_unknown() {
    for line in [false, true] {
        let (mut o, c, target, mut r) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_configured_line_edit(&id, &c, &request(&target), &mut r)
            .unwrap();
        if line {
            o.tighten_line_unit_for_test();
        } else {
            o.configured_line_limits
                .as_mut()
                .unwrap()
                .max_inner_intent_bytes = 0;
        }
        o.run(&id, &mut r).unwrap();
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::ConfiguredLineRejected(
                ConfiguredLineErrorV1::Coordinator(CoordinatorError::Stale)
            ))
        );
        assert!(!o.is_faulted());
    }
}
#[test]
fn interrupted_native_source_install_keeps_lease_until_protected_release_and_never_heals_fault() {
    let (mut o, c, target, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_configured_line_edit(&id, &c, &request(&target), &mut r)
        .unwrap();
    o.publication_fault_after = Some(1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    assert!(o.is_faulted());
    assert_eq!(o.settings_retention_usage().retained, 1);
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retire_settings_retention(&mut r),
        Err(crate::settings::SettingsError::Unavailable)
    );
    assert_eq!(o.release_configured_line_retention(&id, &mut r), Ok(true));
    assert_eq!(o.settings_retention_usage().retained, 0);
    o.retire_settings_retention(&mut r).unwrap();
    assert!(o.is_faulted());
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
}
