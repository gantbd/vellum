use super::*;
fn setup() -> (
    SourceCoordinator,
    Vec<SettingChangeV1>,
    ReadDependencyRequestV3,
    DocumentVersion,
    Recorder,
) {
    let (mut o, source, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_settings_activation(
        crate::settings::SettingsActivationLimitsV1 {
            max_changes: 8,
            max_request_nodes: 64,
            max_request_depth: 32,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    let changes = vec![SettingChangeV1 {
        layer: crate::settings::SettingIdentityV1 {
            authority: "engine".into(),
            id: "target".into(),
            incarnation: 1,
        },
        expected_generation: 1,
        key: "flag".into(),
        operation: crate::settings::SettingOperationV1::Set(
            crate::settings::SettingValueV1::Boolean(true),
        ),
    }];
    let reads = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(source.clone()),
        ))],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    };
    (o, changes, reads, source, r)
}
#[test]
fn full_read_plus_settings_intent_is_charged_before_binding_and_bad_reads_leave_issued() {
    let (mut o, changes, reads, _, mut r) = setup();
    let captured = read_dependency::ReadInput::V3(&reads).copy(&[]).unwrap();
    let settings =
        SettingsRequestCapture::copy(&changes, o.settings_activation.unwrap(), 65536).unwrap();
    let bytes = std::mem::size_of::<SettingsReadIntent>()
        - std::mem::size_of::<SettingsRequestCapture>()
        - std::mem::size_of::<read_dependency::ReadCapture>()
        + captured.logical_bytes().unwrap()
        + settings.retained_bytes();
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    assert_eq!(
        o.submit_settings_change_with_reads(&id, &changes, &reads, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.invocations.limits.max_intent_bytes = bytes;
    o.submit_settings_change_with_reads(&id, &changes, &reads, &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::SettingsCommitted(_))
    ));
    o.invocations.limits.max_intent_bytes = 16384;
    let id = o.issue_invocation(&mut r).unwrap();
    let mut duplicate = reads;
    duplicate.predicates.push(duplicate.predicates[0].clone());
    o.invocations.limits.max_intent_bytes = 1;
    assert_eq!(
        o.submit_settings_change_with_reads(&id, &changes, &duplicate, &mut r),
        Err(InvocationError::Limit)
    );
    o.invocations.limits.max_intent_bytes = 16384;
    assert_eq!(
        o.submit_settings_change_with_reads(&id, &changes, &duplicate, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn combined_preparation_bound_includes_actual_wrapper_and_read_rows_without_leaking_holds() {
    let (mut o, changes, reads, _, mut r) = setup();
    let p = o
        .prepare_settings_change_with_reads(&changes, &reads, &mut r)
        .unwrap();
    let bytes = p.retained_bytes().unwrap();
    drop(p);
    o.settings_activation.as_mut().unwrap().max_plan_bytes = bytes - 1;
    let ordinary = o.prepare_settings_change(&changes, &mut r).unwrap();
    assert!(ordinary.plan.retained_bytes().unwrap() < bytes);
    drop(ordinary);
    assert_eq!(
        o.prepare_settings_change_with_reads(&changes, &reads, &mut r)
            .err(),
        Some(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.settings_activation.as_mut().unwrap().max_plan_bytes = bytes;
    let p = o
        .prepare_settings_change_with_reads(&changes, &reads, &mut r)
        .unwrap();
    assert_eq!(p.retained_bytes().unwrap(), bytes);
    o.commit_settings_change_with_reads(p, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.settings_activation.as_mut().unwrap().max_plan_bytes = 1;
    let mut duplicate = reads;
    duplicate.predicates.push(duplicate.predicates[0].clone());
    assert_eq!(
        o.prepare_settings_change_with_reads(&[], &duplicate, &mut r)
            .err(),
        Some(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    );
}
#[test]
fn read_capture_loss_substitution_and_wrong_refusal_fence_unknown_and_direct_seal_cannot_be_removed()
 {
    for drift in 0..4 {
        let (mut o, mut changes, reads, _, mut r) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_change_with_reads(&id, &changes, &reads, &mut r)
            .unwrap();
        changes[0].operation = crate::settings::SettingOperationV1::Set(
            crate::settings::SettingValueV1::Boolean(false),
        );
        let replacement = o
            .prepare_settings_change_with_reads(&changes, &reads, &mut r)
            .unwrap();
        let slot = o.invocations.slots[id.slot].as_mut().unwrap();
        let Phase::DependentSettingsPending(prepared) = &mut slot.phase else {
            unreachable!()
        };
        match drift {
            0 => prepared.as_mut().unwrap().reads_required = false,
            1 => prepared.as_mut().unwrap().reads.predicates.clear(),
            2 => *prepared = Ok(replacement),
            _ => *prepared = Err(SettingsChangeErrorV2::Reads(CoordinatorError::Capacity)),
        };
        let generation = o.generation();
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
        assert_eq!(o.invocation_usage().faulted_executions, 1);
    }
    for drift in 0..3 {
        let (mut o, changes, reads, _, mut r) = setup();
        let mut p = o
            .prepare_settings_change_with_reads(&changes, &reads, &mut r)
            .unwrap();
        if drift == 1 {
            p.reads = read_dependency::ReadInput::V1(&ReadDependencyRequest {
                predicates: vec![],
                budget: reads.budget,
            })
            .copy(&[])
            .unwrap();
        } else if drift == 0 {
            p.reads_required = false;
        } else {
            p.reads.predicates.clear();
        }
        let generation = o.generation();
        assert_eq!(
            o.commit_settings_change_with_reads(p, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted))
        );
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
    }
}
#[test]
fn changed_noop_reads_reject_but_matching_noops_need_no_unused_publication_capacity() {
    for stale in [false, true] {
        let (mut o, _, reads, source, mut r) = setup();
        let p = o
            .prepare_settings_change_with_reads(&[], &reads, &mut r)
            .unwrap();
        if stale {
            let edit = o
                .prepare(
                    &[DocumentEdits {
                        expected: source,
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
            o.commit(edit, &mut r).unwrap();
        }
        o.generation = u64::MAX;
        let result = o.commit_settings_change_with_reads(p, &mut r);
        if stale {
            assert_eq!(
                result,
                Err(SettingsChangeErrorV2::Reads(CoordinatorError::Stale))
            );
        } else {
            assert!(!result.unwrap().changed);
        }
        assert_eq!(o.generation(), u64::MAX);
        assert!(!o.is_faulted());
    }
}
#[test]
fn interruption_after_actual_settings_install_remains_running_unknown_without_a_receipt() {
    let (mut o, changes, reads, _, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change_with_reads(&id, &changes, &reads, &mut r)
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
    assert_eq!(o.invocation_usage().faulted_executions, 1);
}
