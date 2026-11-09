use super::*;
use crate::settings::*;
fn identity() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "host".into(),
        incarnation: 1,
    }
}
fn profile() -> SettingsActivationLimitsV1 {
    SettingsActivationLimitsV1 {
        max_changes: 8,
        max_request_nodes: 64,
        max_request_depth: 32,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn definition(r: &mut Recorder) -> CompiledSettingsV1 {
    compile_settings_v1(
        &SettingsDefinitionV1 {
            schemas: vec![SettingSchemaV1 {
                key: "unit".into(),
                value_type: SettingTypeV1::Integer { min: 0, max: 8 },
                default: SettingValueV1::Number(1.0),
                domain: SettingDomainV1::Source,
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
            max_schema_depth: 32,
            max_value_nodes: 128,
            max_value_depth: 32,
            max_state_bytes: 65536,
            max_work: 65536,
        },
        r,
    )
    .unwrap()
}
fn setup(install: bool) -> (SourceCoordinator, Recorder) {
    let mut r = Recorder::disabled();
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 65536,
        },
    );
    o.configure_settings_activation(profile(), &mut r).unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    if install {
        o.install_initial_settings(definition(&mut r), &mut r)
            .unwrap();
    }
    (o, r)
}
fn request(value: f64) -> [SettingChangeV1; 1] {
    [SettingChangeV1 {
        layer: identity(),
        expected_generation: 1,
        key: "unit".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(value)),
    }]
}
#[test]
fn descriptor_charges_full_flat_capture_and_refuses_before_binding() {
    for max_work in 0..128 {
        let p = SettingsActivationLimitsV1 {
            max_work,
            ..profile()
        };
        let map = |reverse: bool| {
            let mut rows = vec![
                ("a".into(), SettingValueV1::Null),
                (
                    "b".into(),
                    SettingValueV1::List(vec![SettingValueV1::Number(-0.0)]),
                ),
            ];
            if reverse {
                rows.reverse()
            }
            [SettingChangeV1 {
                operation: SettingOperationV1::Set(SettingValueV1::Map(rows)),
                ..request(2.0).into_iter().next().unwrap()
            }]
        };
        if let Ok(original) = SettingsRequestCapture::copy(&map(false), p, 65536) {
            let retry = SettingsRequestCapture::copy(&map(true), p, 65536).unwrap();
            assert_eq!(original.same(&retry), Ok(true));
        }
    }
    let (mut o, mut r) = setup(true);
    let changes = request(2.0);
    let bytes = SettingsRequestCapture::copy(&changes, profile(), 65536)
        .unwrap()
        .retained_bytes()
        + std::mem::size_of::<SettingsIntent>()
        - std::mem::size_of::<SettingsRequestCapture>();
    assert!(bytes > std::mem::size_of::<SettingChangeV1>());
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    assert_eq!(
        o.submit_settings_change(&id, &changes, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.invocations.limits.max_intent_bytes = bytes;
    o.submit_settings_change(&id, &changes, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::SettingsCommitted(_))
    ));
}
#[test]
fn structural_limits_and_missing_activation_refuse_without_binding_or_publishing() {
    for invalid in 0..4 {
        let (mut o, mut r) = setup(true);
        let id = o.issue_invocation(&mut r).unwrap();
        let mut changes = request(2.0);
        let expected = match invalid {
            0 => {
                changes[0].operation = SettingOperationV1::Set(SettingValueV1::Number(f64::NAN));
                InvocationError::MalformedIntent
            }
            1 => {
                changes[0].operation = SettingOperationV1::Set(SettingValueV1::Map(vec![
                    ("a".into(), SettingValueV1::Null),
                    ("a".into(), SettingValueV1::Null),
                ]));
                InvocationError::MalformedIntent
            }
            2 => {
                let mut value = SettingValueV1::Null;
                for _ in 0..40 {
                    value = SettingValueV1::List(vec![value]);
                }
                changes[0].operation = SettingOperationV1::Set(value);
                InvocationError::Limit
            }
            _ => {
                o.settings_activation = None;
                InvocationError::Unavailable
            }
        };
        let generation = o.generation();
        assert_eq!(
            o.submit_settings_change(&id, &changes, &mut r),
            Err(expected)
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
    }
}
#[test]
fn unavailable_preparation_remains_the_original_refusal_after_initial_installation() {
    let (mut o, mut r) = setup(false);
    let id = o.issue_invocation(&mut r).unwrap();
    let changes = request(2.0);
    o.submit_settings_change(&id, &changes, &mut r).unwrap();
    o.install_initial_settings(definition(&mut r), &mut r)
        .unwrap();
    let generation = o.generation();
    o.submit_settings_change(&id, &changes, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::SettingsRejected(
            SettingsError::Unavailable
        ))
    );
    assert_eq!(o.generation(), generation);
}
#[test]
fn intent_preparation_refusal_or_family_drift_fences_unknown_not_rejected() {
    for drift in 0..4 {
        let (mut o, mut r) = setup(true);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_change(&id, &request(2.0), &mut r)
            .unwrap();
        let replacement = o.prepare_settings_change(&request(3.0), &mut r).unwrap();
        let slot = o.invocations.slots[id.slot].as_mut().unwrap();
        match drift {
            0 => slot.phase = Phase::SettingsPending(Ok(replacement)),
            1 => slot.phase = Phase::SettingsPending(Err(SettingsError::InvalidValue)),
            2 => {
                slot.intent = Some(BoundIntent::Relation {
                    command: RelationCommandV1::CreateWorkspace,
                    profile: None,
                    result: None,
                })
            }
            _ => {
                let Some(BoundIntent::Settings(intent)) = &mut slot.intent else {
                    unreachable!()
                };
                intent.request.profile.max_changes += 1;
            }
        }
        let generation = o.generation();
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.invocation_usage().faulted_executions, 1);
    }
}
#[test]
fn changed_runtime_profile_refuses_current_service_without_refreshing_original_request() {
    let (mut o, mut r) = setup(true);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change(&id, &request(2.0), &mut r)
        .unwrap();
    o.settings_activation.as_mut().unwrap().max_changes += 1;
    o.submit_settings_change(&id, &request(2.0), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::SettingsRejected(SettingsError::Stale))
    );
    assert!(!o.is_faulted());
}
#[test]
fn interruption_after_settings_root_install_leaves_running_unknown_and_no_success_receipt() {
    let (mut o, mut r) = setup(true);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change(&id, &request(2.0), &mut r)
        .unwrap();
    let generation = o.generation();
    o.settings_publication_fault = true;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(
        o.settings
            .as_ref()
            .unwrap()
            .basis()
            .configuration_generation,
        2
    );
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.stop_invocation(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.invocation_usage().faulted_executions, 1);
}
