use super::*;
use crate::coordinator::scoped_definition::tests::*;
#[test]
fn whole_parent_wrapper_precedes_loss_and_failed_admission_binds_nothing() {
    let (mut o, _) = setup();
    let mut r = timed_service_recorder();
    let c = context();
    let common = common(&c);
    let mut f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let d = declaration(true);
    let reads = reads();
    let req = ScopedSettingsDefinitionRequestV1 {
        basis: &b,
        declaration: &d,
        dependencies: &reads,
    };
    let p = o
        .prepare_scoped_settings_definition(&req, &c, &gate(&f), &mut r)
        .unwrap();
    let full = p.retained_bytes().unwrap();
    drop(p);
    let job = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = full - 1;
    f.basis_allowed = Some(false);
    assert_eq!(
        o.submit_scoped_settings_definition(&job, &req, &c, &gate(&f), &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.invocations.limits.max_intent_bytes = full;
    o.submit_scoped_settings_definition(&job, &req, &c, &gate(&f), &mut r)
        .unwrap();
    o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r)
        .unwrap();
    o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r)
        .unwrap();
    assert_service_markers(
        &mut r,
        &[
            (Boundary::InvocationExecution, Disposition::Completed),
            (Boundary::InvocationReplay, Disposition::Completed),
        ],
    );
}
#[test]
fn legitimate_equal_condition_plan_substitution_is_unknown_not_rejected() {
    let (mut o, mut r) = setup();
    let c = context();
    let common = common(&c);
    let f = basis(&common);
    let a = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let d = declaration(true);
    let reads = reads();
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_definition(
        &job,
        &ScopedSettingsDefinitionRequestV1 {
            basis: &a,
            declaration: &d,
            dependencies: &reads,
        },
        &c,
        &gate(&f),
        &mut r,
    )
    .unwrap();
    let p = o
        .prepare_scoped_settings_definition(
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &d,
                dependencies: &reads,
            },
            &c,
            &gate(&f),
            &mut r,
        )
        .unwrap();
    o.invocations.slots[job.slot].as_mut().unwrap().phase = Phase::ScopedDefinitionPending(p);
    let generation = o.generation;
    assert_eq!(
        o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert!(o.is_faulted());
    assert_eq!(o.generation, generation);
    for b in [&a, &b] {
        assert!(o.retire_settings_definition_basis(b, &c, &control(&c), &mut r));
    }
    o.retire_settings_retention(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn actual_post_install_interruption_remains_running_and_protected_release_works() {
    let (mut o, mut r) = setup();
    let c = context();
    let common = common(&c);
    let f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let d = declaration(true);
    let reads = reads();
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_definition(
        &job,
        &ScopedSettingsDefinitionRequestV1 {
            basis: &b,
            declaration: &d,
            dependencies: &reads,
        },
        &c,
        &gate(&f),
        &mut r,
    )
    .unwrap();
    let generation = o.generation;
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r)
        ))
        .is_err()
    );
    assert_eq!(o.generation, generation + 1);
    assert!(o.is_faulted());
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(
        o.invocation_outcome(&job, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.generation, generation + 1);
    assert!(o.retire_settings_definition_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.retire_settings_retention(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn noop_requires_no_unused_publication_capacity_or_unrelated_changed_facet_right() {
    let (mut o, mut r) = setup();
    let c = context();
    let common = common(&c);
    let f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let d = declaration(false);
    let reads = reads();
    let req = ScopedSettingsDefinitionRequestV1 {
        basis: &b,
        declaration: &d,
        dependencies: &reads,
    };
    let mut g = gate(&f);
    g.schema_writable = None;
    g.restrictions_writable = None;
    g.associations_writable = None;
    o.generation = u64::MAX;
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_definition(&job, &req, &c, &g, &mut r)
        .unwrap();
    assert_eq!(
        o.run_scoped_settings_definition(&job, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation, u64::MAX);
    assert!(!o.is_faulted());
}
