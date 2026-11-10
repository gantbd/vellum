use super::*;
use crate::settings::{
    SettingLayerCreateV1, SettingScopeV1, SettingsLayerChangeV1, SettingsLayerLimitsV1,
};
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn new_id() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "new".into(),
        incarnation: 2,
    }
}
fn proposal() -> SettingsLayerChangeV1 {
    SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: new_id(),
            scope: SettingScopeV1::Host,
            values: vec![],
        }],
        remove: vec![],
        targets: fixture::declaration(false).targets,
    }
}
fn facts<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedNativeSettingsLifecycleInvocationV1<'a> {
    TrustedNativeSettingsLifecycleInvocationV1 {
        common,
        profile: limits(),
        layer_registry_writable: Some(true),
        associations_writable: None,
        required_reads: &[],
    }
}
#[test]
fn native_lifecycle_complete_parent_and_original_preparation_identity_are_both_required() {
    for corrupt in [false, true] {
        let (mut o, mut r) = fixture::setup();
        o.configure_settings_layers(limits(), &mut r).unwrap();
        let c = fixture::context();
        let grants = [new_id()];
        let mut common = fixture::common(&c);
        common.writable_layers = Some(&grants);
        let f = facts(&common);
        let changes = proposal();
        let reads = reads();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let input = Input::Lifecycle {
            expected,
            changes: &changes,
        };
        let capture = o
            .capture_native_settings(input, &reads, &c, Facts::Lifecycle(&f))
            .unwrap();
        let other = o.prepare_native_settings(capture, input, &mut r).unwrap();
        let bytes = other.bytes().unwrap();
        let q = NativeSettingsLifecycleRequestV1 {
            expected,
            changes: &changes,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        let before = o.generation();
        if !corrupt {
            drop(other);
            o.invocations.limits.max_intent_bytes = bytes - 1;
            assert_eq!(
                o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r),
                Err(InvocationError::Limit)
            );
            assert_eq!(
                o.invocation_state(&job, &mut r),
                Ok(InvocationState::Issued)
            );
            assert_eq!(o.settings_retention_usage().open_preparations, 0);
            o.invocations.limits.max_intent_bytes = bytes;
            o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r)
                .unwrap();
            o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
                .unwrap();
        } else {
            o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r)
                .unwrap();
            let Phase::NativeSettingsPending(p) =
                &mut o.invocations.slots[job.slot].as_mut().unwrap().phase
            else {
                panic!("pending")
            };
            p.native = other.native;
            assert_eq!(
                o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r),
                Err(InvocationError::Faulted)
            );
            assert!(o.is_faulted());
            assert_eq!(o.generation(), before);
        }
    }
}
#[test]
fn native_lifecycle_identity_needs_no_publication_counter_registry_or_layer_write_grant() {
    let (mut o, mut r) = fixture::setup();
    o.configure_settings_layers(limits(), &mut r).unwrap();
    let c = fixture::context();
    let common = fixture::common(&c);
    let mut f = facts(&common);
    f.layer_registry_writable = None;
    let mut changes = proposal();
    changes.create.clear();
    let reads = reads();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = NativeSettingsLifecycleRequestV1 {
        expected,
        changes: &changes,
        dependencies: &reads,
    };
    o.generation = u64::MAX;
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r)
        .unwrap();
    o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.generation(), u64::MAX);
    assert!(
        matches!(o.invocation_outcome(&job,&mut r),Ok(InvocationOutcome::SettingsLayersCommitted(outcome)) if !outcome.changed)
    );
}
#[test]
fn native_created_result_staging_and_post_install_interruptions_preserve_unknown_and_protected_release()
 {
    for after_install in [false, true] {
        let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
        o.configure_settings_activation(fixture::activation(), &mut r)
            .unwrap();
        o.configure_retention_budget(2_097_152, &mut r).unwrap();
        let mut pool = SettingsRetentionLimitsV1 {
            max_issued: 8,
            max_retained: 8,
            max_snapshot_bytes: 65536,
            max_pool_bytes: usize::MAX,
        };
        pool.max_pool_bytes = pool.required_pool_bytes().unwrap();
        o.configure_settings_retention(pool, &mut r).unwrap();
        o.configure_settings_bases(
            "local",
            SettingsBasisLimitsV1 {
                max_label_bytes: 128,
                max_context_bytes: 4096,
                max_grants: 16,
                max_work: 65536,
            },
            &mut r,
        )
        .unwrap();
        o.configure_settings_layers(limits(), &mut r).unwrap();
        let c = fixture::context();
        let grants = [new_id()];
        let mut common = fixture::common(&c);
        common.writable_layers = Some(&grants);
        let f = facts(&common);
        let changes = proposal();
        let reads = reads();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let q = NativeSettingsLifecycleRequestV1 {
            expected,
            changes: &changes,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        let result = TrustedCreatedLayerResultAdmissionV1 {
            context: &c,
            max_result_bytes: 65536,
            references_allowed: Some(true),
            outcome_allowed: Some(true),
            context_current: Some(true),
        };
        o.submit_native_lifecycle_with_created_results_v1(&job, &q, &c, &f, &result, &mut r)
            .unwrap();
        if after_install {
            o.settings_publication_fault = true;
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
                ))
                .is_err()
            );
        } else {
            o.created_result_fault_after = Some(1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
                ))
                .is_err()
            );
        }
        assert!(o.is_faulted());
        assert_eq!(
            o.settings.as_ref().unwrap().layer_basis().layer_incarnation,
            if after_install { 2 } else { 1 }
        );
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Err(InvocationError::Faulted)
        );
        let release = TrustedCreatedLayerResultReleaseV1 {
            context: &c,
            outcome_readable: Some(true),
            created_result_readable: Some(true),
            created_result_context_current: Some(true),
            historical_layers_readable: Some(true),
        };
        assert!(o.created_layer_results(&job, &release, &mut r).is_err());
        o.forget_created_layer_results(
            &job,
            &SettingsBasisControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            },
            &mut r,
        )
        .unwrap();
    }
}
