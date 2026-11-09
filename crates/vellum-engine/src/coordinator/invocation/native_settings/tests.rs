use super::*;
use crate::coordinator::scoped_definition::tests as fixture;
use crate::settings::{SettingIdentityV1, SettingOperationV1, SettingValueV1};
mod definition;
mod lifecycle;
fn layer() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
fn request() -> [SettingChangeV1; 1] {
    [SettingChangeV1 {
        layer: layer(),
        expected_generation: 1,
        key: "flag".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
    }]
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    }
}
#[test]
fn complete_capture_and_prepared_parent_bounds_precede_admission_and_leave_no_holds() {
    let (mut o, mut r) = fixture::setup();
    let c = fixture::context();
    let layers = [layer()];
    let mut common = fixture::common(&c);
    common.writable_layers = Some(&layers);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let request = request();
    let reads = reads();
    let capture = o
        .capture_native_settings(Input::Values(&request), &reads, &c, Facts::Values(&f))
        .unwrap();
    let p = o
        .prepare_native_settings(capture, Input::Values(&request), &mut r)
        .unwrap();
    let bytes = p.bytes().unwrap();
    drop(p);
    let before = o.generation();
    let job = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    assert_eq!(
        o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert_eq!(o.generation(), before);
    o.invocations.limits.max_intent_bytes = bytes;
    o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
        .unwrap();
    o.run_native_settings_change_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&job, &mut r),
        Ok(InvocationOutcome::SettingsCommitted(_))
    ));
}
#[test]
fn a_legitimate_equal_plan_substitution_is_unknown_not_a_second_authority_source() {
    let (mut o, mut r) = fixture::setup();
    let c = fixture::context();
    let layers = [layer()];
    let mut common = fixture::common(&c);
    common.writable_layers = Some(&layers);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let request = request();
    let reads = reads();
    let capture = o
        .capture_native_settings(Input::Values(&request), &reads, &c, Facts::Values(&f))
        .unwrap();
    let other = o
        .prepare_native_settings(capture, Input::Values(&request), &mut r)
        .unwrap();
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
        .unwrap();
    let before = o.generation();
    let Phase::NativeSettingsPending(p) =
        &mut o.invocations.slots[job.slot].as_mut().unwrap().phase
    else {
        panic!("pending")
    };
    p.native = other.native;
    assert_eq!(
        o.run_native_settings_change_v1(&job, &c, &f, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), before);
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Err(InvocationError::Faulted)
    );
}
#[test]
fn substituted_capture_seal_or_unexpected_preparation_alias_faults_before_publication() {
    for drift in 0..3 {
        let (mut o, mut r) = fixture::setup();
        let c = fixture::context();
        let layers = [layer()];
        let mut common = fixture::common(&c);
        common.writable_layers = Some(&layers);
        let f = TrustedNativeSettingsInvocationV1 {
            common: &common,
            required_reads: &[],
        };
        let request = request();
        let reads = reads();
        let other = o
            .capture_native_settings(Input::Values(&request), &reads, &c, Facts::Values(&f))
            .unwrap();
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
            .unwrap();
        let before = o.generation();
        let Phase::NativeSettingsPending(p) =
            &mut o.invocations.slots[job.slot].as_mut().unwrap().phase
        else {
            panic!("pending")
        };
        let mut alias = None;
        match drift {
            0 => p.capture = Arc::new(other),
            1 => p.seal = Arc::new(other),
            _ => alias = Some(p.native.clone()),
        }
        assert_eq!(
            o.run_native_settings_change_v1(&job, &c, &f, &mut r),
            Err(InvocationError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), before);
        drop(alias);
    }
}
#[test]
fn complete_native_grant_scan_obeys_context_work_bound_including_matched_prefix() {
    for maximum in [8, 9] {
        let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
        o.configure_settings_activation(fixture::activation(), &mut r)
            .unwrap();
        o.configure_settings_bases(
            "local",
            SettingsBasisLimitsV1 {
                max_label_bytes: 128,
                max_context_bytes: 4096,
                max_grants: 8,
                max_work: maximum,
            },
            &mut r,
        )
        .unwrap();
        let c = fixture::context();
        let layers = [layer()];
        let mut common = fixture::common(&c);
        common.writable_layers = Some(&layers);
        let f = TrustedNativeSettingsInvocationV1 {
            common: &common,
            required_reads: &[],
        };
        let request = request();
        let reads = reads();
        let job = o.issue_invocation(&mut r).unwrap();
        let result = o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r);
        if maximum == 8 {
            assert_eq!(result, Err(InvocationError::Limit));
            assert_eq!(
                o.invocation_state(&job, &mut r),
                Ok(InvocationState::Issued)
            );
        } else {
            assert_eq!(result, Ok(InvocationState::Pending));
        }
    }
}
#[test]
fn interruption_after_actual_installation_stays_unknown_and_cannot_be_stopped_or_retried_as_new() {
    let (mut o, mut r) = fixture::setup();
    let c = fixture::context();
    let layers = [layer()];
    let mut common = fixture::common(&c);
    common.writable_layers = Some(&layers);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let request = request();
    let reads = reads();
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
        .unwrap();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_native_settings_change_v1(&job, &c, &f, &mut r)
        ))
        .is_err()
    );
    assert!(o.is_faulted());
    assert_eq!(
        o.settings
            .as_ref()
            .unwrap()
            .read(&layer(), "flag")
            .unwrap()
            .value
            .boolean(),
        Some(true)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.stop_invocation(&job, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r),
        Err(InvocationError::Faulted)
    );
}

#[test]
fn empty_native_job_needs_no_unused_publication_counter_or_layer_basis_permission() {
    let (mut o, mut r) = fixture::setup();
    let c = fixture::context();
    let common = fixture::common(&c);
    assert!(common.basis_allowed.is_none());
    assert!(common.writable_layers.is_none());
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let reads = reads();
    o.generation = u64::MAX;
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_native_settings_change_v1(&job, &[], &reads, &c, &f, &mut r)
        .unwrap();
    o.run_native_settings_change_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert!(
        matches!(o.invocation_outcome(&job,&mut r),Ok(InvocationOutcome::SettingsCommitted(outcome)) if !outcome.changed)
    );
    assert_eq!(o.generation(), u64::MAX);
}
