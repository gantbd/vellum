use super::*;
use crate::coordinator::scoped_definition::tests::{context, control};
use crate::settings::*;
fn id(n: u64) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: n,
    }
}
fn ready(
    maximum: usize,
) -> (
    SourceCoordinator,
    SettingsLayerChangeV1,
    ReadDependencyRequestV3,
    Recorder,
) {
    let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_retention_budget(maximum, &mut r).unwrap();
    o.configure_settings_layers(
        SettingsLayerLimitsV1 {
            max_changes: 8,
            max_created_value_bytes: 16384,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    let q = SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: id(2),
            scope: SettingScopeV1::Host,
            values: vec![],
        }],
        remove: vec![],
        targets: vec![SettingTargetV1 {
            identity: id(1),
            layers: vec![id(1)],
        }],
    };
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    };
    (o, q, reads, r)
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedCreatedLayerResultAdmissionV1<'_> {
    TrustedCreatedLayerResultAdmissionV1 {
        context: c,
        max_result_bytes: 65536,
        references_allowed: Some(true),
        outcome_allowed: Some(true),
        context_current: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedCreatedLayerResultReleaseV1<'_> {
    TrustedCreatedLayerResultReleaseV1 {
        context: c,
        outcome_readable: Some(true),
        created_result_readable: Some(true),
        created_result_context_current: Some(true),
        historical_layers_readable: Some(true),
    }
}
#[test]
fn complete_capture_reply_and_parent_boundaries_are_exact_before_result_or_invocation_admission() {
    let c = context();
    let (mut probe, q, reads, mut r) = ready(usize::MAX);
    let ticket = probe.issue_invocation(&mut r).unwrap();
    let expected = probe.settings_layer_basis(&mut r).unwrap();
    let plan = probe
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    let (hold, charge) = probe
        .prepare_created_hold(&ticket, plan.plan.captured_intent().0, &facts(&c))
        .unwrap();
    let result_bound = hold.capture_bytes.max(hold.reply_bytes);
    assert_eq!(
        charge,
        hold.capture_bytes
            + hold.reply_bytes
            + plan
                .plan
                .captured_intent()
                .0
                .captured_limits()
                .max_state_bytes
    );
    for parent_limit in [false, true] {
        for delta in [-1isize, 0, 1] {
            let (mut o, q, reads, mut r) = ready(if parent_limit {
                charge.checked_add_signed(delta).unwrap()
            } else {
                usize::MAX
            });
            let ticket = o.issue_invocation(&mut r).unwrap();
            let expected = o.settings_layer_basis(&mut r).unwrap();
            let generation = o.generation();
            let mut f = facts(&c);
            if !parent_limit {
                f.max_result_bytes = result_bound.checked_add_signed(delta).unwrap();
            }
            let plan = o
                .prepare_settings_layers(expected, &q, &reads, &mut r)
                .unwrap();
            let result =
                o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &f, &mut r);
            assert_eq!(o.generation(), generation);
            if delta < 0 {
                assert_eq!(result, Err(InvocationError::Limit));
                assert_eq!(o.retention_usage().reserved, 0);
                assert_eq!(
                    o.invocation_state(&ticket, &mut r),
                    Ok(InvocationState::Issued)
                );
            } else {
                assert_eq!(result, Ok(InvocationState::Pending));
                assert_eq!(o.retention_usage().reserved, charge);
                crate::coordinator::shutdown::tests::invocation_funding(
                    &o, &ticket, 1, charge, &mut r,
                );
                o.run(&ticket, &mut r).unwrap();
                assert_eq!(
                    o.created_layer_results(&ticket, &release(&c), &mut r)
                        .unwrap()
                        .created
                        .len(),
                    1
                );
                assert_eq!(
                    o.forget_invocation_outcome(&ticket, &mut r),
                    Err(InvocationError::Unavailable)
                );
                crate::coordinator::shutdown::tests::invocation_funding(
                    &o, &ticket, 1, charge, &mut r,
                );
                o.forget_created_layer_results(&ticket, &control(&c), &mut r)
                    .unwrap();
                o.forget_invocation_outcome(&ticket, &mut r).unwrap();
                crate::coordinator::shutdown::tests::invocation_funding(&o, &ticket, 0, 0, &mut r);
            }
        }
    }
}

#[test]
fn private_staging_and_registration_faults_discard_references_and_leave_the_old_root_while_post_install_stays_unknown()
 {
    for point in 0..3 {
        let (mut o, q, reads, mut r) = ready(usize::MAX);
        let c = context();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let plan = o
            .prepare_settings_layers(expected, &q, &reads, &mut r)
            .unwrap();
        o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &facts(&c), &mut r)
            .unwrap();
        let before = o.settings.as_ref().unwrap().clone();
        let generation = o.generation();
        let charge = o.retention_usage().reserved;
        r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        if point < 2 {
            o.created_result_fault_after = Some(point);
        } else {
            o.settings_publication_fault = true;
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&ticket, &mut r)))
                .is_err()
        );
        assert!(o.is_faulted());
        assert_eq!(o.invocation_usage().faulted_executions, 1);
        assert_eq!(
            Arc::ptr_eq(o.settings.as_ref().unwrap(), &before),
            point < 2
        );
        assert_eq!(o.generation(), generation + u64::from(point == 2));
        assert_eq!(
            o.created_layer_results(&ticket, &release(&c), &mut r).err(),
            Some(InvocationError::Faulted)
        );
        assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
        assert_eq!(o.retention_usage().reserved, charge);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert_eq!(
            events
                .iter()
                .filter(|e| e.boundary == Boundary::CreatedResultsStaging)
                .count(),
            usize::from(point == 2)
        );
        assert!(
            events
                .iter()
                .filter(|e| e.boundary == Boundary::CreatedResultsStaging)
                .all(|e| e.disposition == Disposition::Completed && e.service_time.is_some())
        );
        assert!(
            !events
                .iter()
                .any(|e| e.boundary == Boundary::CreatedResultsPublication)
        );
        assert!(
            events
                .iter()
                .any(|e| e.boundary == Boundary::CreatedResultsRelease
                    && e.disposition == Disposition::Unknown
                    && e.service_time.is_some())
        );
        let State::Held(hold) = &o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .created_results
        else {
            panic!("lost hold")
        };
        assert!(matches!(hold.phase, ResultPhase::Serving));
        assert!(
            o.forget_created_layer_results(&ticket, &control(&c), &mut r)
                .unwrap()
        );
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(
            std::iter::from_fn(|| r.pop_front())
                .any(|e| e.boundary == Boundary::CreatedResultsForget
                    && e.disposition == Disposition::Completed
                    && e.service_time.is_some())
        );
    }
}

#[test]
fn a_substituted_plan_binding_is_an_internal_fault_not_clean_rejection_or_result_publication() {
    let (mut o, q, reads, mut r) = ready(usize::MAX);
    let c = context();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &facts(&c), &mut r)
        .unwrap();
    let State::Held(hold) = &mut o.invocations.slots[ticket.slot]
        .as_mut()
        .unwrap()
        .created_results
    else {
        panic!("missing hold")
    };
    hold.plan = Weak::new();
    let generation = o.generation();
    assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
    assert_eq!(o.generation(), generation);
    assert!(o.is_faulted());
    assert!(
        o.created_layer_results(&ticket, &release(&c), &mut r)
            .is_err()
    );
    assert!(
        o.forget_created_layer_results(&ticket, &control(&c), &mut r)
            .unwrap()
    );
}
