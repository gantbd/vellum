use super::*;

fn selected_targets() -> Vec<BasisSettingTargetV1> {
    vec![
        BasisSettingTargetV1 {
            identity: id("A"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Create { index: 0 },
            ],
        },
        BasisSettingTargetV1 {
            identity: id("B"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Existing(id("right")),
            ],
        },
    ]
}

#[test]
fn basis_selected_removal_uses_the_original_opaque_handle_and_the_existing_publisher() {
    let (mut o, mut r) = ready();
    let c = scoped_basis::context();
    let grants = [new_id(), id("left")];
    let f = scoped_basis::facts(&c, &grants);
    let b = basis(&f);
    let source = o.create(b"A", &mut r).unwrap();
    let lifecycle = o.capture_settings_lifecycle_basis(&c, &b, &mut r).unwrap();
    let selected = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let equal = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let bases = [selected];
    let create = creates();
    let targets = selected_targets();
    let dependencies = reads();
    let request = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &create,
        remove: &[0],
        targets: &targets,
        dependencies: &dependencies,
    };
    let mut facts = gate(&b, &grants);
    facts.selected_basis_facts = Some(&f);
    let ticket = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_basis_settings_lifecycle(&ticket, &request, &c, &facts, &mut r),
        Ok(InvocationState::Pending)
    );
    let substitutes = [equal];
    let substituted = BasisSettingsLifecycleRequestV1 {
        bases: &substitutes,
        ..request
    };
    assert_eq!(
        o.submit_basis_settings_lifecycle(&ticket, &substituted, &c, &facts, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Unavailable));
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    assert_eq!(
        o.run_scoped_settings_lifecycle(&ticket, &c, &facts, &mut r),
        Ok(InvocationState::Finished)
    );
    let generation = o.generation();
    assert_eq!(
        o.submit_basis_settings_lifecycle(&ticket, &request, &c, &facts, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.run_scoped_settings_lifecycle(&ticket, &c, &facts, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
    assert_eq!(read(&o, "A", &mut r), 3.0);
}

#[test]
fn malformed_unused_duplicate_foreign_and_bad_index_selectors_do_not_observe_permission_loss() {
    for case in 0..6 {
        let (mut o, mut r) = ready();
        let c = context();
        let grants = [new_id(), id("left")];
        let common = facts(&c, &grants);
        let f = basis(&common);
        let lifecycle = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let selected = o
            .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
            .unwrap();
        let mut bases = vec![selected.clone()];
        let mut remove = vec![0];
        let mut create = creates();
        let mut targets = selected_targets();
        match case {
            0 => remove.clear(), // unused original handle
            1 => remove[0] = 1,
            2 => {
                let second = o
                    .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
                    .unwrap();
                bases.push(second);
                remove.push(1); // distinct handles, same layer
            }
            3 => {
                let (mut foreign, mut fr) = ready();
                bases[0] = foreign
                    .capture_settings_write_basis(&id("left"), &c, &common, &mut fr)
                    .unwrap();
            }
            4 => create[0].id.clear(),
            5 => targets[0].layers[1] = BasisSettingLayerReferenceV1::Basis { index: 7 },
            _ => unreachable!(),
        }
        let dependencies = reads();
        let q = BasisSettingsLifecycleRequestV1 {
            lifecycle: &lifecycle,
            bases: &bases,
            create: &create,
            remove: &remove,
            targets: &targets,
            dependencies: &dependencies,
        };
        let mut lost = facts(&c, &grants);
        lost.basis_allowed = Some(false);
        let mut lost_role = basis(&common);
        lost_role.basis_allowed = Some(false);
        let mut current = gate(&lost_role, &grants);
        current.selected_basis_facts = Some(&lost);
        let generation = o.generation();
        assert!(
            o.prepare_basis_settings_lifecycle(&q, &c, &current, &mut r)
                .is_err(),
            "case {case}"
        );
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&lifecycle, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(
            o.inspect_settings_write_basis(&selected, &c, &common, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
}

#[test]
fn selected_association_members_need_fresh_basis_authority_but_no_unrelated_registry_grants() {
    let (mut o, mut r) = ready();
    let c = context();
    let grants = [id("left")];
    let common = facts(&c, &grants);
    let role = basis(&common);
    let lifecycle = o
        .capture_settings_lifecycle_basis(&c, &role, &mut r)
        .unwrap();
    let selected = o
        .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
        .unwrap();
    let bases = [selected.clone()];
    let targets = vec![
        BasisSettingTargetV1 {
            identity: id("A"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Basis { index: 0 },
            ],
        },
        BasisSettingTargetV1 {
            identity: id("B"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Basis { index: 0 },
            ],
        },
    ];
    let dependencies = reads();
    let q = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &[],
        remove: &[],
        targets: &targets,
        dependencies: &dependencies,
    };
    let mut f = gate(&role, &[]);
    f.layer_registry_writable = None;
    f.writable_layers = None;
    f.selected_basis_facts = Some(&common);
    let plan = o
        .prepare_basis_settings_lifecycle(&q, &c, &f, &mut r)
        .unwrap();
    assert!(
        o.commit_scoped_settings_lifecycle(plan, &c, &f, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(read(&o, "B", &mut r), 1.0);
    assert_eq!(o.settings_layer_generation(&id("left"), &mut r), Ok(1));
}

#[test]
fn fresh_selected_basis_permission_is_independent_and_observed_only_after_valid_preflight() {
    let (mut o, mut r) = ready();
    let c = context();
    let grants = [new_id(), id("left")];
    let common = facts(&c, &grants);
    let role = basis(&common);
    let lifecycle = o
        .capture_settings_lifecycle_basis(&c, &role, &mut r)
        .unwrap();
    let selected = o
        .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
        .unwrap();
    let bases = [selected.clone()];
    let create = creates();
    let targets = selected_targets();
    let dependencies = reads();
    let q = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &create,
        remove: &[0],
        targets: &targets,
        dependencies: &dependencies,
    };
    let mut f = gate(&role, &grants);
    // Missing facts are a temporary refusal, not recognized loss.
    assert!(
        o.prepare_basis_settings_lifecycle(&q, &c, &f, &mut r)
            .is_err()
    );
    assert_eq!(
        o.inspect_settings_write_basis(&selected, &c, &common, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let mut lost = facts(&c, &grants);
    lost.basis_allowed = Some(false);
    f.selected_basis_facts = Some(&lost);
    assert!(
        o.prepare_basis_settings_lifecycle(&q, &c, &f, &mut r)
            .is_err()
    );
    assert_eq!(
        o.inspect_settings_write_basis(&selected, &c, &common, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&lifecycle, &c, &role, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
}

#[test]
fn selected_basis_service_requires_fresh_bound_facts_and_never_reuses_preparation_grants() {
    for case in 0..4 {
        let (mut o, mut r) = ready();
        let c = context();
        let grants = [new_id(), id("left")];
        let common = facts(&c, &grants);
        let role = basis(&common);
        let lifecycle = o
            .capture_settings_lifecycle_basis(&c, &role, &mut r)
            .unwrap();
        let selected = o
            .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
            .unwrap();
        let bases = [selected.clone()];
        let create = creates();
        let targets = selected_targets();
        let dependencies = reads();
        let q = BasisSettingsLifecycleRequestV1 {
            lifecycle: &lifecycle,
            bases: &bases,
            create: &create,
            remove: &[0],
            targets: &targets,
            dependencies: &dependencies,
        };
        let mut f = gate(&role, &grants);
        f.selected_basis_facts = Some(&common);
        let plan = o
            .prepare_basis_settings_lifecycle(&q, &c, &f, &mut r)
            .unwrap();
        let mut foreign = context();
        foreign.recipient = "different".into();
        let mut current = facts(if case == 2 { &foreign } else { &c }, &grants);
        if case == 1 {
            current.basis_allowed = Some(false);
        }
        if case == 3 {
            current.writable_layers = Some(&[]);
        }
        f.selected_basis_facts = if case == 0 { None } else { Some(&current) };
        let generation = o.generation();
        assert!(
            o.commit_scoped_settings_lifecycle(plan, &c, &f, &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
        assert_eq!(
            o.inspect_settings_write_basis(&selected, &c, &common, &mut r),
            if case == 1 || case == 3 {
                SettingsBasisAvailabilityV1::Unavailable
            } else {
                SettingsBasisAvailabilityV1::Current
            }
        );
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&lifecycle, &c, &role, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
}

#[test]
fn stopped_selected_basis_fences_pending_work_without_retiring_independent_holds() {
    let (mut o, mut r) = ready();
    let c = context();
    let grants = [new_id(), id("left")];
    let common = facts(&c, &grants);
    let role = basis(&common);
    let lifecycle = o
        .capture_settings_lifecycle_basis(&c, &role, &mut r)
        .unwrap();
    let selected = o
        .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
        .unwrap();
    let bases = [selected.clone()];
    let create = creates();
    let targets = selected_targets();
    let dependencies = reads();
    let q = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &create,
        remove: &[0],
        targets: &targets,
        dependencies: &dependencies,
    };
    let mut f = gate(&role, &grants);
    f.selected_basis_facts = Some(&common);
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_basis_settings_lifecycle(&ticket, &q, &c, &f, &mut r)
        .unwrap();
    let mut usage = o.settings_retention_usage();
    assert_eq!(usage.open_preparations, 1);
    let generation = o.generation();
    assert!(o.stop_settings_write_basis(&selected, &c, &scoped_basis::control(&c), &mut r));
    assert_eq!(
        o.run_scoped_settings_lifecycle(&ticket, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    assert!(matches!(
        o.invocation_outcome(&ticket, &mut r),
        Ok(InvocationOutcome::ScopedLifecycleRejected(_))
    ));
    assert_eq!(o.generation(), generation);
    usage.open_preparations = 0; // Service drops its plan, not the independent bases.
    assert_eq!(o.settings_retention_usage(), usage);
    o.retire_invocation(&ticket, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage(), usage);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&lifecycle, &c, &role, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
}

#[test]
fn every_selector_is_structurally_current_before_any_prior_permission_loss_can_be_observed() {
    let (mut o, mut r) = ready();
    let c = context();
    let grants = [id("left"), id("right")];
    let common = facts(&c, &grants);
    let left = o
        .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
        .unwrap();
    let right = o
        .capture_settings_write_basis(&id("right"), &c, &common, &mut r)
        .unwrap();
    set(&mut o, "right", 2.0, &mut r);
    let role = basis(&common);
    let lifecycle = o
        .capture_settings_lifecycle_basis(&c, &role, &mut r)
        .unwrap();
    let bases = [left.clone(), right];
    let targets = [
        BasisSettingTargetV1 {
            identity: id("A"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Basis { index: 0 },
            ],
        },
        BasisSettingTargetV1 {
            identity: id("B"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Basis { index: 1 },
            ],
        },
    ];
    let dependencies = reads();
    let q = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &[],
        remove: &[],
        targets: &targets,
        dependencies: &dependencies,
    };
    let mut lost = facts(&c, &grants);
    lost.basis_allowed = Some(false);
    let mut lost_role = basis(&common);
    lost_role.basis_allowed = Some(false);
    let mut f = gate(&lost_role, &grants);
    f.selected_basis_facts = Some(&lost);
    assert!(matches!(
        o.prepare_basis_settings_lifecycle(&q, &c, &f, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
    ));
    assert_eq!(
        o.inspect_settings_write_basis(&left, &c, &common, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&lifecycle, &c, &role, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
}

#[test]
fn selector_vector_bound_precedes_copying_and_replay_preserves_reference_forms() {
    let (mut o, mut r) = ready();
    let c = context();
    let grants = [id("left")];
    let common = facts(&c, &grants);
    let role = basis(&common);
    let lifecycle = o
        .capture_settings_lifecycle_basis(&c, &role, &mut r)
        .unwrap();
    let selected = o
        .capture_settings_write_basis(&id("left"), &c, &common, &mut r)
        .unwrap();
    let oversized = vec![selected.clone(); 17];
    let bases = [selected];
    let mut targets = vec![
        BasisSettingTargetV1 {
            identity: id("A"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Basis { index: 0 },
            ],
        },
        BasisSettingTargetV1 {
            identity: id("B"),
            layers: vec![
                BasisSettingLayerReferenceV1::Existing(id("host")),
                BasisSettingLayerReferenceV1::Basis { index: 0 },
            ],
        },
    ];
    let dependencies = reads();
    let q = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &oversized,
        create: &[],
        remove: &[],
        targets: &targets,
        dependencies: &dependencies,
    };
    let mut f = gate(&role, &grants);
    f.selected_basis_facts = Some(&common);
    assert!(matches!(
        o.prepare_basis_settings_lifecycle(&q, &c, &f, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    let q = BasisSettingsLifecycleRequestV1 { bases: &bases, ..q };
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_basis_settings_lifecycle(&ticket, &q, &c, &f, &mut r)
        .unwrap();
    targets[0].layers[1] = BasisSettingLayerReferenceV1::Existing(id("left"));
    let q = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &[],
        remove: &[],
        targets: &targets,
        dependencies: &dependencies,
    };
    assert_eq!(
        o.submit_basis_settings_lifecycle(&ticket, &q, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.run_scoped_settings_lifecycle(&ticket, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
}

#[test]
fn empty_selector_noop_needs_no_selected_basis_facts_and_uses_the_same_bounded_diagnostics() {
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let common = facts(&c, &[]);
        let role = basis(&common);
        let lifecycle = o
            .capture_settings_lifecycle_basis(&c, &role, &mut r)
            .unwrap();
        let targets: Vec<_> = unchanged()
            .into_iter()
            .map(|t| BasisSettingTargetV1 {
                identity: t.identity,
                layers: t
                    .layers
                    .into_iter()
                    .map(|r| match r {
                        ScopedSettingLayerReferenceV1::Existing(id) => {
                            BasisSettingLayerReferenceV1::Existing(id)
                        }
                        ScopedSettingLayerReferenceV1::Create { index } => {
                            BasisSettingLayerReferenceV1::Create { index }
                        }
                    })
                    .collect(),
            })
            .collect();
        let dependencies = reads();
        let q = BasisSettingsLifecycleRequestV1 {
            lifecycle: &lifecycle,
            bases: &[],
            create: &[],
            remove: &[],
            targets: &targets,
            dependencies: &dependencies,
        };
        let mut f = gate(&role, &[]);
        f.layer_registry_writable = None;
        f.associations_writable = None;
        f.writable_layers = None;
        let generation = o.generation();
        let usage = o.settings_retention_usage();
        let plan = o
            .prepare_basis_settings_lifecycle(&q, &c, &f, &mut r)
            .unwrap();
        assert_eq!(
            format!("{plan:?}"),
            "PreparedScopedSettingsLifecycleV1(<redacted>)"
        );
        assert!(
            !o.commit_scoped_settings_lifecycle(plan, &c, &f, &mut r)
                .unwrap()
                .changed
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.settings_retention_usage(), usage);
        if capacity == Some(128) {
            use vellum_engine::instrumentation::{Boundary, Disposition};
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::ScopedLifecycleLowering,
                Boundary::ScopedLifecyclePreparation,
                Boundary::ScopedLifecycleValidation,
                Boundary::ScopedLifecyclePublication,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
                );
            }
            assert!(events.iter().all(|e| e.service_time.is_none()));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
