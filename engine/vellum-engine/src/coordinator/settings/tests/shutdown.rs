use super::*;
use crate::coordinator::scoped_definition::tests::{activation, common, context, declaration};
use crate::coordinator::shutdown::tests::install;
#[test]
fn shutdown_inventory_tracks_actual_open_settings_plans_without_publication_or_funding_inference() {
    use crate::coordinator::shutdown::tests::{inspect, inspection_current, inventory_owner};
    let (o, mut r) = ready();
    let initial = inspect(&o, &mut r);
    let original = (o.generation(), o.retention_usage());
    let simple = o.prepare_settings_change(&[], &mut r).unwrap();
    let dependencies = dependencies();
    let reads = o
        .prepare_settings_change_with_reads(&[], &dependencies, &mut r)
        .unwrap();
    let basis = o.settings_layer_basis(&mut r).unwrap();
    let layers = o
        .prepare_settings_layers(
            basis,
            &SettingsLayerChangeV1 {
                create: vec![],
                remove: vec![],
                targets: vec![],
            },
            &dependencies,
            &mut r,
        )
        .unwrap();
    let epochs = o.settings_basis(&mut r).unwrap().interpretation();
    let definition = o
        .prepare_settings_definition(epochs, &declaration(false), &dependencies, &mut r)
        .unwrap();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::SettingsPreparations, &[]);
    assert_eq!(row.retained_records, Some(4));
    assert_eq!(row.reserved_bytes, None);
    assert_eq!(row.held_bytes, None);
    assert_eq!(
        inspection_current(&o, &initial, &mut r),
        Err(CoordinatorError::Stale)
    );
    drop(reads);
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::SettingsPreparations, &[]);
    assert_eq!(row.retained_records, Some(3));
    drop((simple, layers, definition));
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::SettingsPreparations, &[]);
    assert_eq!(row.retained_records, Some(0));
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!((o.generation(), o.retention_usage()), original);
}
#[test]
fn shutdown_inventory_settings_pool_reports_actual_retained_interpretation_and_funding() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r) = ready();
    let _interpretation = o.acquire_settings_interpretation(&mut r).unwrap();
    let before = (o.settings_retention_usage(), o.retention_usage());
    let cut = inspect(&o, &mut r);
    let view = inspect_read(&o, &cut, &mut r);
    let row = view
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::SettingsRetention)
        .unwrap();
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(before.0.retained));
    assert_eq!(row.reserved_bytes, Some(before.0.reserved_pool_bytes));
    assert_eq!((o.settings_retention_usage(), o.retention_usage()), before);
}

fn layer_limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn definition_limits() -> SettingsDefinitionLimitsV1 {
    crate::coordinator::scoped_definition::tests::limits()
}
fn basis_limits() -> SettingsBasisLimitsV1 {
    SettingsBasisLimitsV1 {
        max_label_bytes: 128,
        max_context_bytes: 4096,
        max_grants: 16,
        max_work: 65536,
    }
}
fn retention_limits() -> SettingsRetentionLimitsV1 {
    SettingsRetentionLimitsV1 {
        max_issued: 16,
        max_retained: 16,
        max_snapshot_bytes: 65536,
        max_pool_bytes: 2097152,
    }
}
fn configure(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_settings_activation(activation(), r).unwrap();
    o.configure_settings_layers(layer_limits(), r).unwrap();
    o.configure_settings_definitions(definition_limits(), r)
        .unwrap();
    o.configure_settings_bases("local", basis_limits(), r)
        .unwrap();
    o.configure_settings_retention(retention_limits(), r)
        .unwrap();
}
fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    configure(&mut o, &mut r);
    (o, r)
}
fn dependencies() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 65536,
        },
    }
}
fn unavailable<T>(result: Result<T, SettingsChangeErrorV2>) -> bool {
    matches!(
        result,
        Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable)
            | SettingsChangeErrorV2::Reads(CoordinatorError::Unavailable))
    )
}

#[test]
fn shutdown_refuses_new_settings_profiles_and_retention_but_matching_configuration_is_service() {
    for configured in [false, true] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        o.install_initial_settings(compiled(&mut r), &mut r)
            .unwrap();
        o.configure_retention_budget(usize::MAX, &mut r).unwrap();
        if configured {
            configure(&mut o, &mut r);
        }
        install(&mut o, &mut r);
        let before = (
            o.native_root_identity().unwrap(),
            o.settings_retention_usage(),
            o.retention_usage(),
        );
        let expected = if configured {
            Ok(false)
        } else {
            Err(SettingsError::Unavailable)
        };
        assert_eq!(
            o.configure_settings_activation(activation(), &mut r),
            expected
        );
        assert_eq!(
            o.configure_settings_layers(layer_limits(), &mut r),
            expected
        );
        assert_eq!(
            o.configure_settings_definitions(definition_limits(), &mut r),
            expected
        );
        assert_eq!(
            o.configure_settings_bases("local", basis_limits(), &mut r),
            expected
        );
        assert_eq!(
            o.configure_settings_retention(retention_limits(), &mut r),
            expected
        );
        assert_eq!(
            (
                o.native_root_identity().unwrap(),
                o.settings_retention_usage(),
                o.retention_usage()
            ),
            before
        );
        assert!(!o.is_faulted());
    }
}

#[test]
fn shutdown_rejects_initial_settings_installation_without_latching_an_integrity_fault() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let compiled = compiled(&mut r);
    o.configure_retention_budget(1048576, &mut r).unwrap();
    install(&mut o, &mut r);
    let root = o.native_root_identity().unwrap();
    assert_eq!(
        o.install_initial_settings(compiled, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert!(o.settings_usage().is_none());
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_blocks_settings_captures_but_existing_interpretation_and_all_three_bases_drain() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, mut r) = ready();
        let c = context();
        let layers = [identity()];
        let mut f = common(&c);
        f.basis_allowed = Some(true);
        f.writable_layers = Some(&layers);
        let df = TrustedSettingsDefinitionBasisFactsV1 {
            common: &f,
            definition: definition_limits(),
            basis_allowed: Some(true),
        };
        let lf = TrustedSettingsLifecycleBasisFactsV1 {
            common: &f,
            lifecycle: layer_limits(),
            basis_allowed: Some(true),
        };
        let i = o.acquire_settings_interpretation(&mut r).unwrap();
        let w = o
            .capture_settings_write_basis(&layers[0], &c, &f, &mut r)
            .unwrap();
        let d = o
            .capture_settings_definition_basis(&c, &df, &mut r)
            .unwrap();
        let l = o.capture_settings_lifecycle_basis(&c, &lf, &mut r).unwrap();
        install(&mut o, &mut r);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = (o.settings_retention_usage(), o.retention_usage());
        assert_eq!(
            o.acquire_settings_interpretation(&mut r),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(
            o.capture_settings_write_basis(&layers[0], &c, &f, &mut r),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(
            o.capture_settings_definition_basis(&c, &df, &mut r),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(
            o.capture_settings_lifecycle_basis(&c, &lf, &mut r),
            Err(SettingsError::Unavailable)
        );
        assert_eq!((o.settings_retention_usage(), o.retention_usage()), before);
        assert_eq!(
            o.settings_interpretation_read(&i, &layers[0], "flag", &mut r)
                .unwrap()
                .value
                .boolean(),
            Some(false)
        );
        assert_eq!(
            o.inspect_settings_write_basis(&w, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(
            o.inspect_settings_definition_basis(&d, &c, &df, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&l, &c, &lf, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        let control = crate::coordinator::scoped_definition::tests::control(&c);
        assert!(o.retire_settings_write_basis(&w, &c, &control, &mut r));
        assert!(o.retire_settings_definition_basis(&d, &c, &control, &mut r));
        assert!(o.retire_settings_lifecycle_basis(&l, &c, &control, &mut r));
        o.release_settings_interpretation(&i, &mut r).unwrap();
        assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
        assert!(o.settings_retention_usage().closed);
        assert!(o.retention_usage().reserved > 0);
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for b in [
                Boundary::SettingsInterpretationAcquisition,
                Boundary::SettingsBasisCapture,
                Boundary::DefinitionBasisCapture,
                Boundary::SettingsLifecycleBasisCapture,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == b && e.disposition == Disposition::Refused)
                );
            }
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn shutdown_rejects_changed_and_unchanged_settings_preparation_and_final_admission() {
    for changed in [false, true] {
        let (mut o, mut r) = ready();
        let changes = if changed {
            vec![SettingChangeV1 {
                layer: identity(),
                expected_generation: 1,
                key: "flag".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
            }]
        } else {
            vec![]
        };
        let plan = o.prepare_settings_change(&changes, &mut r).unwrap();
        install(&mut o, &mut r);
        let root = o.native_root_identity().unwrap();
        assert!(matches!(
            o.prepare_settings_change(&changes, &mut r),
            Err(SettingsError::Unavailable)
        ));
        assert_eq!(
            o.commit_settings_change(plan, &mut r),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(o.native_root_identity().unwrap(), root);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
        assert!(!o.is_faulted());
    }
}

#[test]
fn shutdown_blocks_dependency_layer_and_definition_plans_even_when_they_are_unchanged() {
    let (mut o, mut r) = ready();
    let dependencies = dependencies();
    let layers = SettingsLayerChangeV1 {
        create: vec![],
        remove: vec![],
        targets: vec![],
    };
    let layer_basis = o.settings_layer_basis(&mut r).unwrap();
    let epochs = o.settings_basis(&mut r).unwrap().interpretation();
    let declaration = declaration(false);
    let reads = o
        .prepare_settings_change_with_reads(&[], &dependencies, &mut r)
        .unwrap();
    let layer_plan = o
        .prepare_settings_layers(layer_basis, &layers, &dependencies, &mut r)
        .unwrap();
    let definition = o
        .prepare_settings_definition(epochs, &declaration, &dependencies, &mut r)
        .unwrap();
    install(&mut o, &mut r);
    let root = o.native_root_identity().unwrap();
    assert!(unavailable(o.prepare_settings_change_with_reads(
        &[],
        &dependencies,
        &mut r
    )));
    assert!(unavailable(o.prepare_settings_layers(
        layer_basis,
        &layers,
        &dependencies,
        &mut r
    )));
    assert!(unavailable(o.prepare_settings_definition(
        epochs,
        &declaration,
        &dependencies,
        &mut r
    )));
    assert!(unavailable(
        o.commit_settings_change_with_reads(reads, &mut r)
    ));
    assert!(unavailable(o.commit_settings_layers(layer_plan, &mut r)));
    assert!(unavailable(
        o.commit_settings_definition(definition, &mut r)
    ));
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_blocks_scoped_value_definition_and_lifecycle_lowering_and_final_admission() {
    let (mut o, mut r) = ready();
    let c = context();
    let layers = [identity()];
    let mut f = common(&c);
    f.basis_allowed = Some(true);
    f.writable_layers = Some(&layers);
    let df = TrustedSettingsDefinitionBasisFactsV1 {
        common: &f,
        definition: definition_limits(),
        basis_allowed: Some(true),
    };
    let lf = TrustedSettingsLifecycleBasisFactsV1 {
        common: &f,
        lifecycle: layer_limits(),
        basis_allowed: Some(true),
    };
    let bases = [o
        .capture_settings_write_basis(&layers[0], &c, &f, &mut r)
        .unwrap()];
    let d = o
        .capture_settings_definition_basis(&c, &df, &mut r)
        .unwrap();
    let l = o.capture_settings_lifecycle_basis(&c, &lf, &mut r).unwrap();
    let dependencies = dependencies();
    let declaration = declaration(false);
    let changes = [ScopedSettingChangeV1 {
        basis: 0,
        key: "flag".into(),
        operation: SettingOperationV1::Remove,
    }];
    let value = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &dependencies,
    };
    let definition = ScopedSettingsDefinitionRequestV1 {
        basis: &d,
        declaration: &declaration,
        dependencies: &dependencies,
    };
    let lifecycle = ScopedSettingsLifecycleRequestV1 {
        basis: &l,
        create: &[],
        remove: &[],
        targets: &[],
        dependencies: &dependencies,
    };
    let vf = TrustedSettingsMutationFactsV1 {
        basis: &f,
        host_basis_allowed: Some(true),
        required_reads: &[],
    };
    let dg = TrustedSettingsDefinitionMutationFactsV1 {
        basis: &df,
        host_basis_allowed: Some(true),
        schema_writable: Some(true),
        restrictions_writable: Some(true),
        associations_writable: Some(true),
        required_reads: &[],
    };
    let lg = TrustedSettingsLifecycleMutationFactsV1 {
        basis: &lf,
        host_basis_allowed: Some(true),
        layer_registry_writable: Some(true),
        associations_writable: Some(true),
        writable_layers: Some(&layers),
        required_reads: &[],
        selected_basis_facts: None,
    };
    let vp = o
        .prepare_scoped_settings_change(&value, &c, &vf, &mut r)
        .unwrap();
    let dp = o
        .prepare_scoped_settings_definition(&definition, &c, &dg, &mut r)
        .unwrap();
    let lp = o
        .prepare_scoped_settings_lifecycle(&lifecycle, &c, &lg, &mut r)
        .unwrap();
    install(&mut o, &mut r);
    let root = o.native_root_identity().unwrap();
    assert!(unavailable(
        o.prepare_scoped_settings_change(&value, &c, &vf, &mut r)
    ));
    assert!(unavailable(o.prepare_scoped_settings_definition(
        &definition,
        &c,
        &dg,
        &mut r
    )));
    assert!(unavailable(
        o.prepare_scoped_settings_lifecycle(&lifecycle, &c, &lg, &mut r)
    ));
    assert!(unavailable(
        o.commit_scoped_settings_change(vp, &c, &vf, &mut r)
    ));
    assert!(unavailable(
        o.commit_scoped_settings_definition(dp, &c, &dg, &mut r)
    ));
    assert!(unavailable(
        o.commit_scoped_settings_lifecycle(lp, &c, &lg, &mut r)
    ));
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert_eq!(
        o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_keeps_original_settings_invocation_replay_and_settles_publication_refusal() {
    for changed in [false, true] {
        let (mut o, mut r) = ready();
        let changes = if changed {
            vec![SettingChangeV1 {
                layer: identity(),
                expected_generation: 1,
                key: "flag".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
            }]
        } else {
            vec![]
        };
        let ticket = o.issue_invocation(&mut r).unwrap();
        assert_eq!(
            o.submit_settings_change(&ticket, &changes, &mut r),
            Ok(InvocationState::Pending)
        );
        install(&mut o, &mut r);
        let generation = o.generation();
        let funding = o.retention_usage();
        let settings = o.settings_basis(&mut r).unwrap();
        assert_eq!(
            o.submit_settings_change(&ticket, &changes, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.run(&ticket, &mut r), Ok(InvocationState::Finished));
        assert!(matches!(
            o.invocation_outcome(&ticket, &mut r),
            Ok(InvocationOutcome::SettingsRejected(
                SettingsError::Unavailable
            ))
        ));
        assert_eq!(
            o.submit_settings_change(&ticket, &changes, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), funding);
        assert_eq!(o.settings_basis(&mut r).unwrap(), settings);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
        assert!(!o.is_faulted());
    }
}
