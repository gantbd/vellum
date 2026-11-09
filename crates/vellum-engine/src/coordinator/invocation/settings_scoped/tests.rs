use super::*;
#[path = "tests/arguments.rs"]
mod arguments;
use crate::settings::*;
fn layer() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
fn activation() -> SettingsActivationLimitsV1 {
    SettingsActivationLimitsV1 {
        max_changes: 8,
        max_request_nodes: 64,
        max_request_depth: 32,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "reader".into(),
        purpose: "edit".into(),
        destination: "client".into(),
        profile: "v1".into(),
        continuity: "session".into(),
    }
}
fn facts<'a>(
    c: &'a SettingsExecutionContextV1,
    layers: &'a [SettingIdentityV1],
) -> TrustedSettingsBasisFactsV1<'a> {
    TrustedSettingsBasisFactsV1 {
        context: c,
        activation: activation(),
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        settings_writable: Some(true),
        basis_allowed: Some(true),
        writable_layers: Some(layers),
    }
}
fn gate<'a>(basis: &'a TrustedSettingsBasisFactsV1<'a>) -> TrustedSettingsMutationFactsV1<'a> {
    TrustedSettingsMutationFactsV1 {
        basis,
        host_basis_allowed: Some(true),
        required_reads: &[],
    }
}
fn setup() -> (SourceCoordinator, Recorder) {
    setup_with(activation())
}
fn setup_with(profile: SettingsActivationLimitsV1) -> (SourceCoordinator, Recorder) {
    let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_settings_activation(profile, &mut r).unwrap();
    let mut pool = SettingsRetentionLimitsV1 {
        max_issued: 8,
        max_retained: 8,
        max_snapshot_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    pool.max_pool_bytes = pool.required_pool_bytes().unwrap();
    o.configure_retention_budget(pool.max_pool_bytes, &mut r)
        .unwrap();
    o.configure_settings_retention(pool, &mut r).unwrap();
    o.configure_settings_bases(
        "local",
        SettingsBasisLimitsV1 {
            max_label_bytes: 128,
            max_context_bytes: 4096,
            max_grants: 32,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}
fn changes(value: bool) -> [ScopedSettingChangeV1; 1] {
    [ScopedSettingChangeV1 {
        basis: 0,
        key: "flag".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Boolean(value)),
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
fn changed_mandatory_count_refuses_before_copy_or_loss_but_same_count_is_still_validated() {
    for same_count in [false, true] {
        let (mut o, mut r) = setup();
        let c = context();
        let layers = [layer()];
        let mut f = facts(&c, &layers);
        let bases = [o
            .capture_settings_write_basis(&layer(), &c, &f, &mut r)
            .unwrap()];
        let changes = changes(true);
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let registry = o.settings_registry(&mut r).unwrap();
        let interpretation = o.settings_basis(&mut r).unwrap().interpretation();
        let valid = [ReadDependencyV3::ConfigurationState {
            registry: registry.clone(),
            expected: 1,
            interpretation,
        }];
        let invalid = [ReadDependencyV3::ConfigurationState {
            registry,
            expected: 0,
            interpretation,
        }];
        let mut g = gate(&f);
        if same_count {
            g.required_reads = &valid;
        }
        let p = o
            .prepare_scoped_settings_change(&request, &c, &g, &mut r)
            .unwrap();
        f.basis_allowed = Some(false);
        let mut g = gate(&f);
        g.required_reads = &invalid;
        let before = o.generation();
        let result = o.commit_scoped_settings_change(p, &c, &g, &mut r);
        let expected = if same_count {
            SettingsChangeErrorV2::Reads(CoordinatorError::ReadDependency(
                ReadDependencyError::InvalidVersion,
            ))
        } else {
            SettingsChangeErrorV2::Settings(SettingsError::Stale)
        };
        assert_eq!(result, Err(expected));
        assert_eq!(o.generation(), before);
        assert!(!o.is_faulted());
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
}
#[test]
fn complete_parent_wrapper_bound_applies_to_direct_and_submitted_plans_before_loss_observation() {
    let (mut o, mut r) = setup();
    let c = context();
    let layers = [layer()];
    let mut f = facts(&c, &layers);
    let bases = [o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap()];
    let changes = changes(true);
    let reads = reads();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &reads,
    };
    let p = o
        .prepare_scoped_settings_change(&request, &c, &gate(&f), &mut r)
        .unwrap();
    let full = p.retained_bytes().unwrap();
    let inner = p.native.retained_bytes().unwrap();
    drop(p);
    assert!(full > inner);
    let job = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = full - 1;
    f.basis_allowed = Some(false);
    assert!(matches!(
        o.prepare_scoped_settings_change(&request, &c, &gate(&f), &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    assert_eq!(
        o.submit_scoped_settings_change(&job, &request, &c, &gate(&f), &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.invocations.limits.max_intent_bytes = full;
    let p = o
        .prepare_scoped_settings_change(&request, &c, &gate(&f), &mut r)
        .unwrap();
    assert_eq!(p.retained_bytes().unwrap(), full);
    drop(p);
    o.submit_scoped_settings_change(&job, &request, &c, &gate(&f), &mut r)
        .unwrap();
    o.run_scoped_settings_change(&job, &c, &gate(&f), &mut r)
        .unwrap();
    for cap in [full - 1, full] {
        let profile = SettingsActivationLimitsV1 {
            max_plan_bytes: cap,
            ..activation()
        };
        let (mut owner, mut rr) = setup_with(profile);
        let mut facts = facts(&c, &layers);
        facts.activation = profile;
        let bases = [owner
            .capture_settings_write_basis(&layer(), &c, &facts, &mut rr)
            .unwrap()];
        let req = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        // The underlying native mutation fits; only the complete scoped wrapper is at the edge.
        let raw = owner
            .prepare_settings_change(
                &[SettingChangeV1 {
                    layer: layer(),
                    expected_generation: 1,
                    key: "flag".into(),
                    operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
                }],
                &mut rr,
            )
            .unwrap();
        drop(raw);
        let result = owner.prepare_scoped_settings_change(&req, &c, &gate(&facts), &mut rr);
        if cap < full {
            assert!(matches!(
                result,
                Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
            ));
        } else {
            assert_eq!(result.unwrap().retained_bytes().unwrap(), full);
        }
    }
}
#[test]
fn private_native_read_or_capture_substitution_faults_before_publishing_or_observing_loss() {
    for kind in 0..4 {
        let (mut o, mut r) = setup();
        let c = context();
        let layers = [layer()];
        let f = facts(&c, &layers);
        let bases = [o
            .capture_settings_write_basis(&layer(), &c, &f, &mut r)
            .unwrap()];
        let original = changes(true);
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &original,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_settings_change(&job, &request, &c, &gate(&f), &mut r)
            .unwrap();
        let alternate = changes(false);
        let replacement = o
            .prepare_scoped_settings_change(
                &ScopedSettingsRequestV1 {
                    bases: &bases,
                    changes: &alternate,
                    dependencies: &reads,
                },
                &c,
                &gate(&f),
                &mut r,
            )
            .unwrap();
        let slot = o.invocations.slots[job.slot].as_mut().unwrap();
        let Phase::ScopedSettingsPending(p) = &mut slot.phase else {
            unreachable!()
        };
        match kind {
            0 => p.native.reads_required = false,
            1 => p.native.read_count = 1,
            2 => p.native.settings = replacement.native.settings,
            _ => p.capture = replacement.capture,
        }
        let before = o.generation();
        assert_eq!(
            o.run_scoped_settings_change(&job, &c, &gate(&f), &mut r),
            Err(InvocationError::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), before);
        assert!(matches!(
            o.invocations.slots[job.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
    }
}
#[test]
fn interrupted_actual_scoped_publication_stays_unknown_and_keeps_basis_until_protected_discharge() {
    let (mut o, mut r) = setup();
    let c = context();
    let layers = [layer()];
    let f = facts(&c, &layers);
    let bases = [o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap()];
    let changes = changes(true);
    let reads = reads();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_change(&job, &request, &c, &gate(&f), &mut r)
        .unwrap();
    let before = o.generation();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_scoped_settings_change(&job, &c, &gate(&f), &mut r)
        ))
        .is_err()
    );
    assert_eq!(o.generation(), before + 1);
    assert!(o.is_faulted());
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(o.settings_retention_usage().retained, 1);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert_eq!(
        o.run_scoped_settings_change(&job, &c, &gate(&f), &mut r),
        Err(InvocationError::Faulted)
    );
    let ctrl = SettingsBasisControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert!(o.retire_settings_write_basis(&bases[0], &c, &ctrl, &mut r));
    assert!(o.is_faulted());
}
#[test]
fn valid_empty_and_noop_scoped_jobs_need_no_unused_publication_counter() {
    for empty in [false, true] {
        let (mut o, mut r) = setup();
        let c = context();
        let layers = [layer()];
        let f = facts(&c, &layers);
        let bases = if empty {
            vec![]
        } else {
            vec![
                o.capture_settings_write_basis(&layer(), &c, &f, &mut r)
                    .unwrap(),
            ]
        };
        let changes = if empty {
            vec![]
        } else {
            vec![ScopedSettingChangeV1 {
                basis: 0,
                key: "flag".into(),
                operation: SettingOperationV1::Remove,
            }]
        };
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        o.generation = u64::MAX;
        o.submit_scoped_settings_change(&job, &request, &c, &gate(&f), &mut r)
            .unwrap();
        o.run_scoped_settings_change(&job, &c, &gate(&f), &mut r)
            .unwrap();
        assert!(matches!(
            o.invocation_outcome(&job, &mut r),
            Ok(InvocationOutcome::ScopedSettingsCommitted(
                ScopedSettingsUpdateV1 { changed: false }
            ))
        ));
        assert_eq!(o.generation(), u64::MAX);
        assert!(!o.is_faulted());
    }
}
#[test]
fn integrity_fault_during_direct_or_submitted_preflight_latches_owner_without_binding_a_job() {
    use crate::coordinator::settings::RetentionPool;
    for direct in [false, true] {
        let (mut o, mut r) = setup();
        let c = context();
        let layers = [layer()];
        let f = facts(&c, &layers);
        let bases = [o
            .capture_settings_write_basis(&layer(), &c, &f, &mut r)
            .unwrap()];
        let changes = changes(true);
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        let (mut other, _) = setup();
        let forged = other.retention.reserve(0).unwrap();
        let RetentionPool::Active(pool) = &mut o.settings_retention else {
            unreachable!()
        };
        let original = std::mem::replace(&mut pool.reservation, forged);
        let generation = o.generation();
        if direct {
            assert!(matches!(
                o.prepare_scoped_settings_change(&request, &c, &gate(&f), &mut r),
                Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted))
            ));
        } else {
            assert_eq!(
                o.submit_scoped_settings_change(&job, &request, &c, &gate(&f), &mut r),
                Err(InvocationError::Faulted)
            );
        }
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
        assert!(matches!(
            o.invocations.slots[job.slot].as_ref().unwrap().phase,
            Phase::Issued(_)
        ));
        let RetentionPool::Active(pool) = &mut o.settings_retention else {
            unreachable!()
        };
        pool.reservation = original;
        let ctrl = SettingsBasisControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        assert!(o.retire_settings_write_basis(&bases[0], &c, &ctrl, &mut r));
        assert!(o.is_faulted());
    }
}
#[test]
fn complete_layer_grant_scan_obeys_the_native_work_ceiling_without_closing_a_basis() {
    let profile = SettingsActivationLimitsV1 {
        max_work: 16,
        ..activation()
    };
    let (mut o, mut r) = setup_with(profile);
    let c = context();
    let mut layers: Vec<_> = (0..16)
        .map(|n| SettingIdentityV1 {
            authority: "engine".into(),
            id: format!("other{n}"),
            incarnation: 1,
        })
        .collect();
    layers.push(layer());
    let mut f = facts(&c, &layers);
    f.activation = profile;
    let bases = [o
        .capture_settings_write_basis(&layer(), &c, &f, &mut r)
        .unwrap()];
    let changes = [ScopedSettingChangeV1 {
        basis: 0,
        key: "flag".into(),
        operation: SettingOperationV1::Remove,
    }];
    let reads = reads();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &reads,
    };
    assert!(matches!(
        o.prepare_scoped_settings_change(&request, &c, &gate(&f), &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    assert_eq!(
        o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let short = [layer()];
    f.writable_layers = Some(&short);
    let p = o
        .prepare_scoped_settings_change(&request, &c, &gate(&f), &mut r)
        .unwrap();
    assert!(
        !o.commit_scoped_settings_change(p, &c, &gate(&f), &mut r)
            .unwrap()
            .changed
    );
}
