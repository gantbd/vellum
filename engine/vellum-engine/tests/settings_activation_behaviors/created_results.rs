use super::scoped_basis::{context, facts};
use super::*;

#[test]
fn f06_receipt_assembly_returns_the_same_borrowed_created_references_without_new_write_authority() {
    let (mut o, mut r) = ready();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = super::observation_support::Profile {
        epoch: 7,
        maximum: 4096,
        capabilities: InvocationReceiptCapabilitiesV1 {
            created_settings: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = proposal();
    let read = reads();
    let plan = o
        .prepare_settings_layers(expected, &q, &read, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&id, plan, &admission(&c), &mut r)
        .unwrap();
    let mut observer = profile.open(&mut o, &id, &c, &mut r);
    o.run(&id, &mut r).unwrap();
    let native = release(&c);
    let original = o
        .created_layer_results(&id, &native, &mut r)
        .unwrap()
        .created[0]
        .reference
        .clone();
    let InvocationReceiptV1::CreatedSettings(out) = profile
        .query(
            &o,
            &mut observer,
            InvocationReceiptRequestV1::CreatedSettings(&native),
            &c,
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(out.invocation, id);
    assert_eq!(out.created[0].reference, original);
}

#[test]
fn f06_receipt_assembly_requires_the_whole_created_reference_frame_even_for_borrowed_delivery() {
    let (mut o, mut r) = ready();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = super::observation_support::Profile {
        epoch: 7,
        maximum: std::mem::size_of::<InvocationOutcomeProjectionV1>(),
        capabilities: InvocationReceiptCapabilitiesV1 {
            created_settings: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = proposal();
    let read = reads();
    let plan = o
        .prepare_settings_layers(expected, &q, &read, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&id, plan, &admission(&c), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
    assert_eq!(
        profile.try_open(&mut o, &id, &c, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
}

pub(super) fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = setup();
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
    o.configure_settings_layers(
        SettingsLayerLimitsV1 {
            max_changes: 16,
            max_created_value_bytes: 65536,
            max_request_bytes: 65536,
            max_plan_bytes: 1_048_576,
            max_work: 1_048_576,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}

#[test]
fn current_release_is_all_or_nothing_context_bound_and_never_discloses_native_identity() {
    let (mut o, mut r) = ready();
    let c = context();
    let q = proposal();
    let reads = reads();
    let id = o.issue_invocation(&mut r).unwrap();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&id, plan, &admission(&c), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let reference = o
        .created_layer_results(&id, &release(&c), &mut r)
        .unwrap()
        .created[0]
        .reference
        .clone();
    for index in 0..8 {
        let mut f = release(&c);
        let fact = if index % 2 == 0 { None } else { Some(false) };
        match index / 2 {
            0 => f.outcome_readable = fact,
            1 => f.created_result_readable = fact,
            2 => f.created_result_context_current = fact,
            3 => f.historical_layers_readable = fact,
            _ => unreachable!(),
        }
        assert!(o.created_layer_results(&id, &f, &mut r).is_err());
    }
    let mut foreign = context();
    foreign.recipient = "different".into();
    assert!(
        o.created_layer_results(&id, &release(&foreign), &mut r)
            .is_err()
    );
    let (other, mut rr) = ready();
    assert!(
        other
            .created_layer_results(&id, &release(&c), &mut rr)
            .is_err()
    );
    let result = o.created_layer_results(&id, &release(&c), &mut r).unwrap();
    assert_eq!(result.created[0].reference, reference);
    assert_eq!(format!("{result:?}"), "CreatedLayerResultV1(<redacted>)");
    assert_eq!(
        format!("{reference:?}"),
        "CreatedSettingLayerReferenceV1(<opaque>)"
    );
}

#[test]
fn historical_reference_is_not_write_authority_and_forget_does_not_revoke_a_separate_fresh_basis() {
    let (mut o, mut r) = ready();
    let c = context();
    let q = proposal();
    let reads = reads();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &admission(&c), &mut r)
        .unwrap();
    o.run(&ticket, &mut r).unwrap();
    let reference = o
        .created_layer_results(&ticket, &release(&c), &mut r)
        .unwrap()
        .created[0]
        .reference
        .clone();
    let denied = facts(&c, &[]);
    assert!(
        o.capture_settings_write_basis_from_created(&reference, &c, &denied, &mut r)
            .is_err()
    );
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    let grants = [SettingIdentityV1 {
        incarnation: 2,
        ..id("new")
    }];
    let f = facts(&c, &grants);
    let basis = o
        .capture_settings_write_basis_from_created(&reference, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
    assert!(
        o.forget_created_layer_results(&ticket, &scoped_basis::control(&c), &mut r)
            .unwrap()
    );
    assert!(
        o.capture_settings_write_basis_from_created(&reference, &c, &f, &mut r)
            .is_err()
    );
    assert_eq!(
        o.inspect_settings_write_basis(&basis, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
}

#[test]
fn removal_and_same_label_recreation_do_not_retarget_a_historical_reference() {
    let (mut o, mut r) = ready();
    let c = context();
    let q = proposal();
    let reads = reads();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &admission(&c), &mut r)
        .unwrap();
    o.run(&ticket, &mut r).unwrap();
    let reference = o
        .created_layer_results(&ticket, &release(&c), &mut r)
        .unwrap()
        .created[0]
        .reference
        .clone();
    let mut q = proposal();
    q.remove.push(SettingLayerRemoveV1 {
        identity: SettingIdentityV1 {
            incarnation: 2,
            ..id("new")
        },
        expected_generation: 1,
    });
    q.create[0].identity.incarnation = 3;
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.commit_settings_layers(plan, &mut r).unwrap();
    let grants = [
        SettingIdentityV1 {
            incarnation: 2,
            ..id("new")
        },
        SettingIdentityV1 {
            incarnation: 3,
            ..id("new")
        },
    ];
    let f = facts(&c, &grants);
    assert!(
        o.capture_settings_write_basis_from_created(&reference, &c, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    assert_eq!(
        o.created_layer_results(&ticket, &release(&c), &mut r)
            .unwrap()
            .created[0]
            .reference,
        reference
    );
}

#[test]
fn refusal_and_stop_never_expose_references_or_refund_the_hold_without_protected_discharge() {
    for stale in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let q = proposal();
        let reads = reads();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let before = o.retention_usage().reserved;
        let plan = o
            .prepare_settings_layers(expected, &q, &reads, &mut r)
            .unwrap();
        o.submit_prepared_settings_layers_with_created_results(
            &ticket,
            plan,
            &admission(&c),
            &mut r,
        )
        .unwrap();
        let held = o.retention_usage().reserved;
        assert_eq!(
            o.forget_created_layer_results(&ticket, &scoped_basis::control(&c), &mut r),
            Err(InvocationError::Pending)
        );
        if stale {
            set(&mut o, "left", 2.0, &mut r);
            o.run(&ticket, &mut r).unwrap();
        } else {
            o.stop_invocation(&ticket, &mut r).unwrap();
        }
        assert!(
            o.created_layer_results(&ticket, &release(&c), &mut r)
                .is_err()
        );
        assert_eq!(o.retention_usage().reserved, held);
        assert!(o.retire_invocation(&ticket, &mut r).is_err());
        let mut control = scoped_basis::control(&c);
        control.delivery_drained = Some(false);
        assert!(
            o.forget_created_layer_results(&ticket, &control, &mut r)
                .is_err()
        );
        assert_eq!(o.retention_usage().reserved, held);
        assert!(
            o.forget_created_layer_results(&ticket, &scoped_basis::control(&c), &mut r)
                .unwrap()
        );
        assert_eq!(o.retention_usage().reserved, before);
        o.retire_invocation(&ticket, &mut r).unwrap();
    }
}

#[test]
fn failed_optional_preparation_leaves_the_ticket_issued_and_allows_explicit_ordinary_fallback() {
    for case in 0..4 {
        let (mut o, mut r) = ready();
        let c = context();
        let q = proposal();
        let reads = reads();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let before = o.retention_usage();
        let mut f = admission(&c);
        match case {
            0 => f.max_result_bytes = 0,
            1 => f.references_allowed = Some(false),
            2 => f.outcome_allowed = None,
            3 => f.context_current = Some(false),
            _ => unreachable!(),
        }
        let plan = o
            .prepare_settings_layers(expected, &q, &reads, &mut r)
            .unwrap();
        assert!(
            o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &f, &mut r)
                .is_err()
        );
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Issued)
        );
        assert_eq!(o.retention_usage(), before);
        o.submit_settings_layers(&ticket, expected, &q, &reads, &mut r)
            .unwrap();
        o.run(&ticket, &mut r).unwrap();
        assert!(
            o.created_layer_results(&ticket, &release(&c), &mut r)
                .is_err()
        );
    }
}

#[test]
fn scoped_and_basis_selected_plans_publish_the_same_result_facet_after_source_only_progress() {
    for selected in [false, true] {
        let (mut o, _) = ready();
        let mut r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(
                vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        let c = context();
        let common = facts(&c, &[]);
        let lifecycle_limits = SettingsLayerLimitsV1 {
            max_changes: 16,
            max_created_value_bytes: 65536,
            max_request_bytes: 65536,
            max_plan_bytes: 1_048_576,
            max_work: 1_048_576,
        };
        let role = TrustedSettingsLifecycleBasisFactsV1 {
            common: &common,
            lifecycle: lifecycle_limits,
            basis_allowed: Some(true),
        };
        let lifecycle = o
            .capture_settings_lifecycle_basis(&c, &role, &mut r)
            .unwrap();
        let grants = [SettingIdentityV1 {
            incarnation: 2,
            ..id("new")
        }];
        let basis_grants = [id("left")];
        let selected_facts = facts(&c, &basis_grants);
        let mut f = TrustedSettingsLifecycleMutationFactsV1 {
            basis: &role,
            host_basis_allowed: Some(true),
            layer_registry_writable: Some(true),
            associations_writable: None,
            writable_layers: Some(&grants),
            required_reads: &[],
            selected_basis_facts: None,
        };
        let source = o.create(b"A", &mut r).unwrap();
        let creates = [ScopedSettingLayerCreateV1 {
            id: "new".into(),
            scope: SettingScopeV1::Workspace,
            values: vec![],
        }];
        let dependencies = reads();
        let native = proposal();
        let plan = if selected {
            let bases = [o
                .capture_settings_write_basis(&id("left"), &c, &selected_facts, &mut r)
                .unwrap()];
            f.selected_basis_facts = Some(&selected_facts);
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
                        BasisSettingLayerReferenceV1::Existing(id("right")),
                    ],
                },
            ];
            o.prepare_basis_settings_lifecycle(
                &BasisSettingsLifecycleRequestV1 {
                    lifecycle: &lifecycle,
                    bases: &bases,
                    create: &creates,
                    remove: &[],
                    targets: &targets,
                    dependencies: &dependencies,
                },
                &c,
                &f,
                &mut r,
            )
            .unwrap()
        } else {
            let targets: Vec<_> = native
                .targets
                .into_iter()
                .map(|t| ScopedSettingTargetV1 {
                    identity: t.identity,
                    layers: t
                        .layers
                        .into_iter()
                        .map(ScopedSettingLayerReferenceV1::Existing)
                        .collect(),
                })
                .collect();
            o.prepare_scoped_settings_lifecycle(
                &ScopedSettingsLifecycleRequestV1 {
                    basis: &lifecycle,
                    create: &creates,
                    remove: &[],
                    targets: &targets,
                    dependencies: &dependencies,
                },
                &c,
                &f,
                &mut r,
            )
            .unwrap()
        };
        let ticket = o.issue_invocation(&mut r).unwrap();
        o.submit_prepared_scoped_lifecycle_with_created_results(
            &ticket,
            plan,
            &admission(&c),
            &mut r,
        )
        .unwrap();
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
        assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Unavailable));
        o.run_scoped_settings_lifecycle(&ticket, &c, &f, &mut r)
            .unwrap();
        let result = o
            .created_layer_results(&ticket, &release(&c), &mut r)
            .unwrap();
        assert_eq!(result.status, CreatedLayerResultStatusV1::Applied);
        assert_eq!(result.created.len(), 1);
        assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
        assert_eq!(read(&o, "A", &mut r), 1.0);
        use vellum_engine::instrumentation::Boundary;
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        for boundary in [
            Boundary::CreatedResultsAdmission,
            Boundary::CreatedResultsStaging,
            Boundary::CreatedResultsPublication,
            Boundary::CreatedResultsRelease,
        ] {
            let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
            assert!(!matching.is_empty());
            assert!(matching.iter().all(|e| e.service_time.is_some()));
        }
    }
}

#[test]
fn empty_creation_noop_returns_an_empty_unchanged_result_without_issuing_versions() {
    let (mut o, mut r) = ready();
    let c = context();
    let mut q = proposal();
    q.create.clear();
    let reads = reads();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &admission(&c), &mut r)
        .unwrap();
    let generation = o.generation();
    o.run(&ticket, &mut r).unwrap();
    let result = o
        .created_layer_results(&ticket, &release(&c), &mut r)
        .unwrap();
    assert_eq!(result.status, CreatedLayerResultStatusV1::Unchanged);
    assert!(result.created.is_empty());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap(), expected);
}

#[test]
fn scoped_stop_intent_and_actual_stopped_settlement_preserve_result_funding_until_protected_cleanup()
 {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let lifecycle = SettingsLayerLimitsV1 {
        max_changes: 16,
        max_created_value_bytes: 65536,
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    };
    let role = TrustedSettingsLifecycleBasisFactsV1 {
        common: &common,
        lifecycle,
        basis_allowed: Some(true),
    };
    let basis = o
        .capture_settings_lifecycle_basis(&c, &role, &mut r)
        .unwrap();
    let grants = [SettingIdentityV1 {
        incarnation: 2,
        ..id("new")
    }];
    let f = TrustedSettingsLifecycleMutationFactsV1 {
        basis: &role,
        host_basis_allowed: Some(true),
        layer_registry_writable: Some(true),
        associations_writable: None,
        writable_layers: Some(&grants),
        required_reads: &[],
        selected_basis_facts: None,
    };
    let create = [ScopedSettingLayerCreateV1 {
        id: "new".into(),
        scope: SettingScopeV1::Workspace,
        values: vec![],
    }];
    let targets: Vec<_> = proposal()
        .targets
        .into_iter()
        .map(|t| ScopedSettingTargetV1 {
            identity: t.identity,
            layers: t
                .layers
                .into_iter()
                .map(ScopedSettingLayerReferenceV1::Existing)
                .collect(),
        })
        .collect();
    let reads = reads();
    let request = ScopedSettingsLifecycleRequestV1 {
        basis: &basis,
        create: &create,
        remove: &[],
        targets: &targets,
        dependencies: &reads,
    };
    let before = o.retention_usage().reserved;
    let plan = o
        .prepare_scoped_settings_lifecycle(&request, &c, &f, &mut r)
        .unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_prepared_scoped_lifecycle_with_created_results(&ticket, plan, &admission(&c), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
    let generation = o.generation();
    let retained = o.settings_retention_usage().retained;
    let control = TrustedInvocationControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    };
    o.request_invocation_stop(&ticket, &control, &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Pending)
    );
    assert!(
        o.forget_created_layer_results(&ticket, &scoped_basis::control(&c), &mut r)
            .is_err()
    );
    o.run(&ticket, &mut r).unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage().reserved, held);
    assert!(
        o.created_layer_results(&ticket, &release(&c), &mut r)
            .is_err()
    );
    o.forget_created_layer_results(&ticket, &scoped_basis::control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, before);
    assert_eq!(o.settings_retention_usage().retained, retained);
}

#[test]
fn optional_result_diagnostics_are_bounded_content_free_and_do_not_change_authoritative_results() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for mode in 0..4 {
        let capacity = match mode {
            0 => None,
            3 => Some(1),
            _ => Some(128),
        };
        let (mut o, _) = ready();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let c = context();
        let q = proposal();
        let reads = reads();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let plan = o
            .prepare_settings_layers(expected, &q, &reads, &mut r)
            .unwrap();
        o.submit_prepared_settings_layers_with_created_results(
            &ticket,
            plan,
            &admission(&c),
            &mut r,
        )
        .unwrap();
        o.run(&ticket, &mut r).unwrap();
        let held = o.retention_usage();
        let mut denied = release(&c);
        denied.created_result_readable = None;
        assert!(o.created_layer_results(&ticket, &denied, &mut r).is_err());
        assert_eq!(o.retention_usage(), held);
        let reference = o
            .created_layer_results(&ticket, &release(&c), &mut r)
            .unwrap()
            .created[0]
            .reference
            .clone();
        assert_eq!(
            o.created_layer_results(&ticket, &release(&c), &mut r)
                .unwrap()
                .created[0]
                .reference,
            reference
        );
        let grants = [SettingIdentityV1 {
            incarnation: 2,
            ..id("new")
        }];
        let f = facts(&c, &grants);
        o.capture_settings_write_basis_from_created(&reference, &c, &f, &mut r)
            .unwrap();
        o.forget_created_layer_results(&ticket, &scoped_basis::control(&c), &mut r)
            .unwrap();
        assert_eq!(o.settings_layer_generation(&grants[0], &mut r), Ok(1));
        assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::CreatedResultsAdmission,
                Boundary::CreatedResultsStaging,
                Boundary::CreatedResultsPublication,
                Boundary::CreatedResultsRelease,
                Boundary::CreatedResultsForget,
                Boundary::CreatedResultsBasisCapture,
            ] {
                let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
                assert!(
                    matching
                        .iter()
                        .any(|e| e.disposition == Disposition::Completed)
                );
                assert!(
                    matching
                        .iter()
                        .all(|e| e.service_time.is_some() == (mode == 2))
                );
            }
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::CreatedResultsRelease
                        && e.disposition == Disposition::Refused)
            );
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
fn proposal() -> SettingsLayerChangeV1 {
    SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: SettingIdentityV1 {
                incarnation: 2,
                ..id("new")
            },
            scope: SettingScopeV1::Workspace,
            values: vec![],
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
fn admission(c: &SettingsExecutionContextV1) -> TrustedCreatedLayerResultAdmissionV1<'_> {
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
fn optional_created_results_release_only_after_success_and_replay_reuses_original_references() {
    let (mut o, mut r) = ready();
    let c = context();
    let q = proposal();
    let reads = reads();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let before = o.retention_usage().reserved;
    let plan = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    o.submit_prepared_settings_layers_with_created_results(&ticket, plan, &admission(&c), &mut r)
        .unwrap();
    assert!(o.retention_usage().reserved > before);
    assert!(
        o.created_layer_results(&ticket, &release(&c), &mut r)
            .is_err()
    );
    assert_eq!(o.run(&ticket, &mut r), Ok(InvocationState::Finished));
    let result = o
        .created_layer_results(&ticket, &release(&c), &mut r)
        .unwrap();
    assert_eq!(result.status, CreatedLayerResultStatusV1::Applied);
    assert_eq!(result.created.len(), 1);
    assert_eq!(result.created[0].index, 0);
    let reference = result.created[0].reference.clone();
    let generation = o.generation();
    o.submit_settings_layers(&ticket, expected, &q, &reads, &mut r)
        .unwrap();
    o.run(&ticket, &mut r).unwrap();
    assert_eq!(
        o.created_layer_results(&ticket, &release(&c), &mut r)
            .unwrap()
            .created[0]
            .reference,
        reference
    );
    assert_eq!(o.generation(), generation);
    assert!(o.forget_invocation_outcome(&ticket, &mut r).is_err());
    let control = scoped_basis::control(&c);
    assert!(
        o.forget_created_layer_results(&ticket, &control, &mut r)
            .unwrap()
    );
    assert_eq!(o.retention_usage().reserved, before);
    assert!(
        !o.forget_created_layer_results(&ticket, &control, &mut r)
            .unwrap()
    );
    let mut no_op = proposal();
    no_op.create.clear();
    let current = o.settings_layer_basis(&mut r).unwrap();
    let prepared = o
        .prepare_settings_layers(current, &no_op, &reads, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_prepared_settings_layers_with_created_results(
            &ticket,
            prepared,
            &admission(&c),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(o.retention_usage().reserved, before);
    assert!(
        o.created_layer_results(&ticket, &release(&c), &mut r)
            .is_err()
    );
    o.forget_invocation_outcome(&ticket, &mut r).unwrap();
    o.retire_invocation(&ticket, &mut r).unwrap();
}
