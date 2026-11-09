use super::*;
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 16,
        max_created_value_bytes: 65536,
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = setup();
    o.configure_settings_layers(limits(), &mut r).unwrap();
    (o, r)
}
fn layer_id(n: u64) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "extra".into(),
        incarnation: n,
    }
}
fn request() -> SettingsLayerChangeV1 {
    SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: layer_id(2),
            scope: SettingScopeV1::Workspace,
            values: vec![(
                "unit".into(),
                SettingContributionV1::Set(SettingValueV1::Number(3.0)),
            )],
        }],
        remove: vec![],
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
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn prepare(
    o: &SourceCoordinator,
    q: &SettingsLayerChangeV1,
    r: &mut Recorder,
) -> Result<PreparedSettingsLayersV1, SettingsChangeErrorV2> {
    o.prepare_settings_layers(o.settings_layer_basis(r).unwrap(), q, &reads(), r)
}
fn apply(
    o: &mut SourceCoordinator,
    q: &SettingsLayerChangeV1,
    r: &mut Recorder,
) -> SettingsLayerOutcomeV1 {
    let p = prepare(o, q, r).unwrap();
    o.commit_settings_layers(p, r).unwrap()
}
#[test]
fn create_remove_and_reassociate_is_one_transition_preserving_surviving_authored_layers() {
    let (mut o, mut r) = ready();
    set(&mut o, "left", 2.0, &mut r);
    let a = o.settings_target(&id("A"), &mut r).unwrap();
    let before = o.generation();
    let mut q = request();
    q.remove.push(SettingLayerRemoveV1 {
        identity: id("left"),
        expected_generation: 2,
    });
    q.targets[0].layers = vec![id("host"), layer_id(2)];
    let result = apply(&mut o, &q, &mut r);
    assert!(result.changed && result.associations_changed);
    assert_eq!(result.created_layers, 1);
    assert_eq!(result.removed_layers, 1);
    assert_eq!(result.changed_domains, 1);
    assert_eq!(o.generation(), before + 1);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
    assert_eq!(
        o.settings_layer_generation(&id("left"), &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(o.settings_layer_generation(&layer_id(2), &mut r), Ok(1));
    assert_eq!(o.settings_layer_generation(&id("right"), &mut r), Ok(1));
    assert_eq!(o.settings_target(&id("A"), &mut r).unwrap(), a);
    assert_eq!(read(&o, "A", &mut r), 3.0);
}
#[test]
fn unattached_creation_and_removal_never_refund_incarnation_or_change_effective_domains() {
    let (mut o, mut r) = ready();
    let mut q = request();
    let before = o.settings_basis(&mut r).unwrap();
    let created = apply(&mut o, &q, &mut r);
    assert_eq!(created.changed_domains, 0);
    assert!(!created.associations_changed);
    assert_eq!(
        created.basis.association_generation,
        before.association_generation
    );
    q.create.clear();
    q.remove.push(SettingLayerRemoveV1 {
        identity: layer_id(2),
        expected_generation: 1,
    });
    assert_eq!(apply(&mut o, &q, &mut r).changed_domains, 0);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
    q.remove.clear();
    q.create = request().create;
    assert!(prepare(&o, &q, &mut r).is_err());
    q.create[0].identity.incarnation = 3;
    assert_eq!(apply(&mut o, &q, &mut r).layer_incarnation, 3);
}
#[test]
fn removing_an_unrelated_layer_cannot_retarget_or_spuriously_stale_a_value_plan() {
    let (mut o, mut r) = ready();
    let mut q = request();
    q.create.clear();
    q.targets[0].layers = vec![id("host")];
    apply(&mut o, &q, &mut r); // left is now unused, preceding right in storage
    let p = o
        .prepare_settings_change(
            &[change(
                "right",
                1,
                SettingOperationV1::Set(SettingValueV1::Number(4.0)),
            )],
            &mut r,
        )
        .unwrap();
    q.remove.push(SettingLayerRemoveV1 {
        identity: id("left"),
        expected_generation: 1,
    });
    apply(&mut o, &q, &mut r);
    assert!(o.commit_settings_change(p, &mut r).unwrap().changed);
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(read(&o, "B", &mut r), 4.0);
}
#[test]
fn multiple_creation_is_contiguous_order_bound_and_failed_preparation_spends_no_identity() {
    let (mut o, mut r) = ready();
    let mut q = request();
    q.create.push(SettingLayerCreateV1 {
        identity: layer_id(3),
        scope: SettingScopeV1::Host,
        values: vec![],
    });
    q.create.swap(0, 1);
    assert!(prepare(&o, &q, &mut r).is_err());
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
    q.create.swap(0, 1);
    let p = prepare(&o, &q, &mut r).unwrap();
    drop(p);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
    assert_eq!(apply(&mut o, &q, &mut r).layer_incarnation, 3);
    assert_eq!(o.settings_layer_generation(&layer_id(2), &mut r), Ok(1));
    assert_eq!(o.settings_layer_generation(&layer_id(3), &mut r), Ok(1));
}
#[test]
fn malformed_or_incoherent_candidates_leave_no_authored_or_identity_prefix() {
    for kind in 0..12 {
        let (o, mut r) = ready();
        let mut q = request();
        match kind {
            0 => q.create[0].identity.incarnation = 1,
            1 => q.create[0].identity.incarnation = 3,
            2 => q.create[0].identity.authority = "foreign".into(),
            3 => q.create[0].values[0].0 = "unknown".into(),
            4 => {
                q.create[0].values[0].1 = SettingContributionV1::Set(SettingValueV1::Boolean(true))
            }
            5 => {
                q.create.clear();
                q.remove.push(SettingLayerRemoveV1 {
                    identity: id("left"),
                    expected_generation: 1,
                });
            }
            6 => {
                q.remove.push(SettingLayerRemoveV1 {
                    identity: id("left"),
                    expected_generation: 2,
                });
            }
            7 => {
                q.remove.push(SettingLayerRemoveV1 {
                    identity: id("missing"),
                    expected_generation: 1,
                });
            }
            8 => q.targets[0].layers.push(id("missing")),
            9 => q.targets[0].layers.push(id("host")),
            10 => {
                q.remove.push(SettingLayerRemoveV1 {
                    identity: id("left"),
                    expected_generation: 1,
                });
                q.remove.push(SettingLayerRemoveV1 {
                    identity: id("left"),
                    expected_generation: 1,
                });
            }
            _ => {
                q.create[0]
                    .values
                    .push(("unit".into(), SettingContributionV1::Reset));
            }
        }
        let basis = o.settings_layer_basis(&mut r).unwrap();
        let generation = o.generation();
        assert!(prepare(&o, &q, &mut r).is_err(), "kind {kind}");
        assert_eq!(o.settings_layer_basis(&mut r).unwrap(), basis);
        assert_eq!(o.generation(), generation);
        assert_eq!(read(&o, "A", &mut r), 1.0);
        assert_eq!(o.settings_usage().unwrap().layers, 3);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
    }
}
#[test]
fn whole_settings_proposals_stale_on_configuration_progress_but_not_unread_source_progress() {
    for changed in ["source", "read-source", "settings"] {
        let (mut o, mut r) = ready();
        let source = o.create(b"S", &mut r).unwrap();
        let mut req = reads();
        if changed == "read-source" {
            req.predicates
                .push(ReadDependencyV3::Base(ReadDependencyV2::Base(
                    ReadDependencyV1::Source(source.clone()),
                )));
        }
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let p = o
            .prepare_settings_layers(expected, &request(), &req, &mut r)
            .unwrap();
        if changed == "settings" {
            set(&mut o, "right", 2.0, &mut r);
        } else {
            super::settings_reads::source_edit(&mut o, &source, &mut r);
        }
        let before = o.generation();
        let result = o.commit_settings_layers(p, &mut r);
        match changed {
            "source" => {
                assert!(result.unwrap().changed);
                assert_eq!(o.generation(), before + 1);
            }
            "read-source" => assert_eq!(
                result,
                Err(SettingsChangeErrorV2::Reads(CoordinatorError::Stale))
            ),
            _ => assert_eq!(
                result,
                Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
            ),
        }
        assert!(!o.is_faulted());
    }
}
#[test]
fn shared_lifecycle_job_replay_is_historical_and_stop_forget_retire_do_not_rearm() {
    for stop in [false, true] {
        let (mut o, mut r) = ready();
        let q = request();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let ticket = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_layers(&ticket, expected, &q, &reads(), &mut r)
            .unwrap();
        assert_eq!(o.settings_retention_usage().open_preparations, 1);
        if stop {
            o.stop_invocation(&ticket, &mut r).unwrap();
        } else {
            o.run(&ticket, &mut r).unwrap();
        }
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
        if !stop {
            assert!(
                matches!(o.invocation_outcome(&ticket, &mut r), Ok(InvocationOutcome::SettingsLayersCommitted(result)) if result.layer_incarnation == 2)
            );
        }
        set(&mut o, "left", 2.0, &mut r);
        let generation = o.generation();
        assert_eq!(
            o.submit_settings_layers(&ticket, expected, &q, &reads(), &mut r),
            Ok(InvocationState::Finished)
        );
        o.forget_invocation_outcome(&ticket, &mut r).unwrap();
        o.run(&ticket, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.settings_layer_basis(&mut r).unwrap().layer_incarnation,
            if stop { 1 } else { 2 }
        );
        o.retire_invocation(&ticket, &mut r).unwrap();
        assert_eq!(
            o.submit_settings_layers(&ticket, expected, &q, &reads(), &mut r),
            Err(InvocationError::Unavailable)
        );
    }
}
#[test]
fn repeated_create_remove_compacts_flat_storage_and_preserves_incarnation_across_other_producers() {
    let (mut o, mut r) = ready();
    o.configure_settings_definitions(
        SettingsDefinitionLimitsV1 {
            max_request_bytes: 65536,
            max_plan_bytes: 1_048_576,
            max_work: 1_048_576,
        },
        &mut r,
    )
    .unwrap();
    let baseline = o.settings_usage().unwrap();
    for n in 2..34 {
        let mut q = request();
        q.create[0].identity.incarnation = n;
        apply(&mut o, &q, &mut r);
        set(&mut o, "left", f64::from(2 + (n % 6) as u32), &mut r);
        let mut d = super::definition::declaration();
        d.schemas[0].sensitive = n % 2 == 0;
        let e = o.settings_basis(&mut r).unwrap().interpretation();
        let p = o
            .prepare_settings_definition(e, &d, &reads(), &mut r)
            .unwrap();
        o.commit_settings_definition(p, &mut r).unwrap();
        assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, n);
        q.create.clear();
        q.remove.push(SettingLayerRemoveV1 {
            identity: layer_id(n),
            expected_generation: 1,
        });
        apply(&mut o, &q, &mut r);
        assert_eq!(o.settings_usage().unwrap().layers, baseline.layers);
        assert!(o.settings_usage().unwrap().value_nodes <= baseline.value_nodes + 1);
    }
}
#[test]
fn no_creation_uses_no_created_value_allowance_and_changed_profiles_are_not_silently_reconfigured()
{
    let (mut o, mut r) = setup();
    let mut p = limits();
    p.max_created_value_bytes = 0;
    assert_eq!(o.configure_settings_layers(p, &mut r), Ok(true));
    assert_eq!(o.configure_settings_layers(p, &mut r), Ok(false));
    assert_eq!(
        o.configure_settings_layers(limits(), &mut r),
        Err(SettingsError::Unavailable)
    );
    assert!(matches!(
        prepare(&o, &request(), &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    let mut q = request();
    q.create.clear();
    let generation = o.generation();
    assert!(!apply(&mut o, &q, &mut r).changed);
    assert_eq!(o.generation(), generation);
}
#[test]
fn removed_values_remain_in_old_interpretations_and_preparation_holds_block_pool_retirement() {
    let (mut o, mut r) = ready();
    set(&mut o, "left", 4.0, &mut r);
    let mut pool = SettingsRetentionLimitsV1 {
        max_issued: 4,
        max_retained: 4,
        max_snapshot_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    pool.max_pool_bytes = pool.required_pool_bytes().unwrap();
    o.configure_retention_budget(pool.max_pool_bytes, &mut r)
        .unwrap();
    o.configure_settings_retention(pool, &mut r).unwrap();
    let old = o.acquire_settings_interpretation(&mut r).unwrap();
    let mut q = request();
    q.remove.push(SettingLayerRemoveV1 {
        identity: id("left"),
        expected_generation: 2,
    });
    q.targets[0].layers = vec![id("host"), layer_id(2)];
    apply(&mut o, &q, &mut r);
    assert_eq!(
        o.settings_interpretation_read(&old, &id("A"), "unit", &mut r)
            .unwrap()
            .value
            .number(),
        Some(4.0)
    );
    assert_eq!(read(&o, "A", &mut r), 3.0);
    o.release_settings_interpretation(&old, &mut r).unwrap();
    q.create.clear();
    q.remove.clear();
    let p = prepare(&o, &q, &mut r).unwrap();
    assert_eq!(
        o.retire_settings_retention(&mut r),
        Err(SettingsError::Unavailable)
    );
    drop(p);
    assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
    assert!(matches!(
        prepare(&o, &q, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable))
    ));
}
#[test]
fn lifecycle_signals_are_optional_bounded_and_do_not_change_native_results() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        o.configure_settings_layers(limits(), &mut r).unwrap();
        apply(&mut o, &request(), &mut r);
        let mut q = request();
        q.create.clear();
        let p = prepare(&o, &q, &mut r).unwrap();
        set(&mut o, "right", 2.0, &mut r);
        assert_eq!(
            o.commit_settings_layers(p, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SettingsLayerConfiguration,
                Boundary::SettingsLayerPreparation,
                Boundary::SettingsLayerValidation,
                Boundary::SettingsLayerPublication,
            ] {
                assert!(records.iter().any(|record| record.boundary == boundary));
            }
            assert!(records.iter().any(|record| record.boundary
                == Boundary::SettingsLayerPublication
                && record.disposition == Disposition::Refused));
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn original_native_refusal_and_exact_number_bits_remain_bound_after_other_settings_progress() {
    for valid in [false, true] {
        let (mut o, mut r) = ready();
        let mut q = request();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        q.create[0].values[0].1 = SettingContributionV1::Set(if valid {
            SettingValueV1::Number(-0.0)
        } else {
            SettingValueV1::Boolean(true)
        });
        let ticket = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_layers(&ticket, expected, &q, &reads(), &mut r)
            .unwrap();
        if valid {
            q.create[0].values[0].1 = SettingContributionV1::Set(SettingValueV1::Number(0.0));
            assert_eq!(
                o.submit_settings_layers(&ticket, expected, &q, &reads(), &mut r),
                Err(InvocationError::IntentMismatch)
            );
            q.create[0].values[0].1 = SettingContributionV1::Set(SettingValueV1::Number(-0.0));
        }
        set(&mut o, "left", 2.0, &mut r);
        o.run(&ticket, &mut r).unwrap();
        assert!(
            matches!(o.invocation_outcome(&ticket, &mut r), Ok(InvocationOutcome::SettingsLayersRejected(SettingsChangeErrorV2::Settings(error))) if *error == if valid { SettingsError::Stale } else { SettingsError::InvalidValue })
        );
        assert_eq!(
            o.submit_settings_layers(&ticket, expected, &q, &reads(), &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
    }
}
