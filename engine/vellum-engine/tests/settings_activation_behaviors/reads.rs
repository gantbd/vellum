use super::*;
fn request(predicates: Vec<ReadDependencyV3>) -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates,
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 4096,
        },
    }
}
fn domain(
    o: &SourceCoordinator,
    name: &str,
    d: SettingDomainV1,
    r: &mut Recorder,
) -> ReadDependencyV3 {
    let target = o.settings_target(&id(name), r).unwrap();
    let g = o.settings_generations(&target, r).unwrap();
    let expected = match d {
        SettingDomainV1::Source => g.source,
        SettingDomainV1::Serialization => g.serialization,
        SettingDomainV1::Analysis => g.analysis,
    };
    ReadDependencyV3::ConfigurationDomain {
        target,
        domain: d,
        expected,
        interpretation: o.settings_basis(r).unwrap().interpretation(),
    }
}
fn state(o: &SourceCoordinator, r: &mut Recorder) -> ReadDependencyV3 {
    let basis = o.settings_basis(r).unwrap();
    ReadDependencyV3::ConfigurationState {
        registry: o.settings_registry(r).unwrap(),
        expected: basis.configuration_generation,
        interpretation: basis.interpretation(),
    }
}
fn intent(v: &DocumentVersion, unchanged: bool) -> CompoundIntent {
    CompoundIntent {
        source: SourceIntent {
            writes: vec![DocumentEdits {
                expected: v.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: if unchanged { "A" } else { "a" }.into(),
                }],
            }],
            reads: vec![],
        },
        annotations: vec![],
    }
}
fn mutate(o: &mut SourceCoordinator, layer: &str, key: &str, value: f64, r: &mut Recorder) {
    let generation = o.settings_layer_generation(&id(layer), r).unwrap();
    let p = o
        .prepare_settings_change(
            &[SettingChangeV1 {
                layer: id(layer),
                expected_generation: generation,
                key: key.into(),
                operation: SettingOperationV1::Set(SettingValueV1::Number(value)),
            }],
            r,
        )
        .unwrap();
    o.commit_settings_change(p, r).unwrap();
}
#[test]
fn finite_configuration_conjunctions_match_independent_domain_and_metadata_frame_oracle() {
    for mask in 0..128 {
        for layer in ["left", "right", "host"] {
            for (column, key) in [(0, "unit"), (1, "codec"), (2, "analysis")] {
                for effective in [false, true] {
                    let (mut o, mut r) = setup_with(profile(), true);
                    let source = o.create(b"A", &mut r).unwrap();
                    let edit = intent(&source, false);
                    let mut predicates = Vec::new();
                    for (target_index, target) in ["A", "B"].into_iter().enumerate() {
                        for (domain_index, d) in [
                            SettingDomainV1::Source,
                            SettingDomainV1::Serialization,
                            SettingDomainV1::Analysis,
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let bit = target_index * 3 + domain_index;
                            if mask & (1 << bit) != 0 {
                                predicates.push(domain(&o, target, d, &mut r));
                            }
                        }
                    }
                    if mask & 64 != 0 {
                        predicates.push(state(&o, &mut r));
                    }
                    let req = request(predicates);
                    let plan = o
                        .prepare_compound_with_reads_v3(&edit, &req, &mut r)
                        .unwrap();
                    mutate(
                        &mut o,
                        layer,
                        key,
                        if effective { 2.0 } else { 1.0 },
                        &mut r,
                    );
                    let affected_a = layer != "right";
                    let affected_b = layer != "left";
                    let stale = mask & 64 != 0
                        || effective
                            && ((affected_a && mask & (1 << column) != 0)
                                || (affected_b && mask & (1 << (column + 3)) != 0));
                    let basis = o.settings_basis(&mut r).unwrap();
                    let generation = o.generation();
                    let advice = o.check_read_dependencies_v3(&[], &req, &mut r);
                    let result = o.commit(plan, &mut r);
                    if stale {
                        assert_eq!(advice, Err(CoordinatorError::Stale));
                        assert_eq!(result, Err(CoordinatorError::Stale));
                        assert_eq!(o.generation(), generation);
                        assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "A");
                    } else {
                        advice.unwrap();
                        let result = result.unwrap();
                        assert_eq!(result.changes.len(), 1);
                        assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "a");
                    }
                    assert_eq!(o.settings_basis(&mut r).unwrap(), basis);
                    assert!(!o.is_faulted());
                }
            }
        }
    }
}
#[test]
fn all_interpretation_epochs_and_positive_safe_versions_are_required_for_each_new_kind() {
    let (o, mut r) = setup_with(profile(), true);
    for whole in [false, true] {
        for field in 0..4 {
            for value in [0, 2, 9_007_199_254_740_991, 9_007_199_254_740_992] {
                let mut predicate = if whole {
                    state(&o, &mut r)
                } else {
                    domain(&o, "A", SettingDomainV1::Source, &mut r)
                };
                let (expected, epochs) = match &mut predicate {
                    ReadDependencyV3::ConfigurationDomain {
                        expected,
                        interpretation,
                        ..
                    }
                    | ReadDependencyV3::ConfigurationState {
                        expected,
                        interpretation,
                        ..
                    } => (expected, interpretation),
                    _ => unreachable!(),
                };
                *match field {
                    0 => expected,
                    1 => &mut epochs.schema,
                    2 => &mut epochs.restriction,
                    _ => &mut epochs.association,
                } = value;
                assert_eq!(
                    o.check_read_dependencies_v3(&[], &request(vec![predicate]), &mut r),
                    Err(if value > 0 && value <= 9_007_199_254_740_991 {
                        CoordinatorError::Stale
                    } else {
                        CoordinatorError::ReadDependency(ReadDependencyError::InvalidVersion)
                    })
                );
            }
        }
    }
}
#[test]
fn configuration_aba_and_changed_noops_refuse_without_settings_history_or_source_prefix() {
    let (mut o, mut r) = setup_with(profile(), true);
    let source = o.create(b"A", &mut r).unwrap();
    let req = request(vec![domain(&o, "A", SettingDomainV1::Source, &mut r)]);
    let edit = intent(&source, true);
    let plan = o
        .prepare_compound_with_reads_v3(&edit, &req, &mut r)
        .unwrap();
    mutate(&mut o, "left", "unit", 2.0, &mut r);
    mutate(&mut o, "left", "unit", 1.0, &mut r);
    let generation = o.generation();
    assert_eq!(o.commit(plan, &mut r), Err(CoordinatorError::Stale));
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "A");
    assert_eq!(o.history_usage().retained_groups, 0);
}
#[test]
fn scoped_mixed_source_workspace_and_configuration_reads_share_one_original_root() {
    use vellum_engine::source::Replacement;
    let (mut o, mut r) = setup_with(profile(), true);
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 8,
            max_live: 8,
            max_memberships: 8,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let w = o.create_workspace(&mut r).unwrap();
    let workspace = w.workspace.clone();
    let a = o.create(b"A", &mut r).unwrap();
    let b = o.create(b"B", &mut r).unwrap();
    let req = request(vec![
        ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(b.clone()))),
        ReadDependencyV3::Base(ReadDependencyV2::WorkspaceMembership {
            workspace: workspace.clone(),
            expected: w.generation,
        }),
        domain(&o, "A", SettingDomainV1::Source, &mut r),
    ]);
    let scopes = [SourceOperationScope {
        expected: a.clone(),
        replacement_ranges: vec![ByteRange { start: 0, end: 1 }],
        insertion_sites: vec![],
    }];
    let budget = SourceScopeBudget {
        max_scopes: 8,
        max_records: 8,
        max_work: 4096,
    };
    let edit = intent(&a, false);
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_compound_with_reads_v3(&invocation, &edit, &req, &scopes, budget, &mut r)
        .unwrap();
    let peer = o
        .prepare(
            &[DocumentEdits {
                expected: b,
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "B".into(),
                    inserted: "b".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(peer, &mut r).unwrap();
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
    );
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "A");
    let direct = o
        .prepare_scoped_compound_with_reads_v3(
            &edit,
            &request(vec![domain(&o, "A", SettingDomainV1::Source, &mut r)]),
            &scopes,
            budget,
            &mut r,
        )
        .unwrap();
    o.commit(direct, &mut r).unwrap();
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "a");
}
#[test]
fn matching_replay_keeps_old_configuration_predicates_and_old_profile_vocabulary_closed() {
    let (mut o, mut r) = setup_with(profile(), true);
    let source = o.create(b"A", &mut r).unwrap();
    let edit = intent(&source, false);
    let req = request(vec![domain(&o, "A", SettingDomainV1::Source, &mut r)]);
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_compound_with_reads_v3(&invocation, &edit, &req, &mut r)
        .unwrap();
    o.run(&invocation, &mut r).unwrap();
    let published = o.generation();
    mutate(&mut o, "left", "unit", 2.0, &mut r);
    o.submit_compound_with_reads_v3(&invocation, &edit, &req, &mut r)
        .unwrap();
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(o.generation(), published + 1);
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "a");
    let fresh = request(vec![domain(&o, "A", SettingDomainV1::Source, &mut r)]);
    assert_eq!(
        o.submit_compound_with_reads_v3(&invocation, &edit, &fresh, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let invocation = o.issue_invocation(&mut r).unwrap();
    let empty = request(vec![]);
    o.submit_compound_with_reads_v3(&invocation, &edit, &empty, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_compound_with_reads_v2(
            &invocation,
            &edit,
            &ReadDependencyRequestV2 {
                predicates: vec![],
                budget: empty.budget
            },
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn foreign_handles_duplicate_subjects_and_limits_refuse_without_binding_or_observation_grants() {
    let (mut o, mut r) = setup_with(profile(), true);
    let (foreign, mut fr) = setup_with(profile(), true);
    for p in [
        state(&foreign, &mut fr),
        domain(&foreign, "A", SettingDomainV1::Source, &mut fr),
    ] {
        assert_eq!(
            o.check_read_dependencies_v3(&[], &request(vec![p]), &mut r),
            Err(CoordinatorError::Unavailable)
        );
    }
    let predicate = domain(&o, "A", SettingDomainV1::Source, &mut r);
    let duplicate = request(vec![predicate.clone(), predicate]);
    assert_eq!(
        o.check_read_dependencies_v3(&[], &duplicate, &mut r),
        Err(CoordinatorError::ReadDependency(
            ReadDependencyError::DuplicateSubject
        ))
    );
    let source = o.create(b"A", &mut r).unwrap();
    let edit = intent(&source, false);
    let invocation = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_compound_with_reads_v3(&invocation, &edit, &duplicate, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(
        o.invocation_state(&invocation, &mut r),
        Ok(InvocationState::Issued)
    );
    let mut req = request(vec![state(&o, &mut r)]);
    req.budget.max_predicates = 0;
    assert_eq!(
        o.check_read_dependencies_v3(&[], &req, &mut r),
        Err(CoordinatorError::ReadDependency(ReadDependencyError::Limit))
    );
    req.budget.max_predicates = 1;
    req.budget.max_work = 0;
    assert_eq!(
        o.check_read_dependencies_v3(&[], &req, &mut r),
        Err(CoordinatorError::ReadDependency(ReadDependencyError::Limit))
    );
}
#[test]
fn empty_v3_conjunction_needs_no_configuration_or_workspace_module_and_capture_is_optional() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let mut o = SourceCoordinator::configured(CoordinatorConfig {
            limits: CoordinatorLimits {
                max_document_incarnations: 4,
                max_participants: 4,
                max_staged_bytes: 128,
                source: Limits {
                    max_source_bytes: 128,
                    max_edits: 8,
                    max_edit_payload_bytes: 256,
                },
            },
            invocation: None,
            history: None,
            resources: None,
            search: None,
            annotations: Some(AnnotationLimits {
                max_issued: 4,
                max_records: 4,
                max_body_bytes: 128,
                max_retained_body_bytes: 1024,
                max_mapping_steps: 4096,
                max_live_snapshots: 4,
                max_snapshot_bytes: 4096,
            }),
        });
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        assert_eq!(o.settings_registry(&mut r), Err(SettingsError::Unavailable));
        let (mut configured, _) = setup_with(profile(), true);
        let v = configured.create(b"A", &mut r).unwrap();
        let guarded = request(vec![
            domain(&configured, "A", SettingDomainV1::Source, &mut r),
            state(&configured, &mut r),
        ]);
        configured
            .check_read_dependencies_v3(&[], &guarded, &mut r)
            .unwrap();
        let p = configured
            .prepare_compound_with_reads_v3(&intent(&v, false), &guarded, &mut r)
            .unwrap();
        mutate(&mut configured, "left", "unit", 2.0, &mut r);
        assert_eq!(
            configured.check_read_dependencies_v3(&[], &guarded, &mut r),
            Err(CoordinatorError::Stale)
        );
        assert_eq!(configured.commit(p, &mut r), Err(CoordinatorError::Stale));
        assert_eq!(
            configured.read(&v.document, &mut r).unwrap().source.text(),
            "A"
        );
        let req = request(vec![]);
        o.check_read_dependencies_v3(&[], &req, &mut r).unwrap();
        let source = o.create(b"A", &mut r).unwrap();
        let edit = intent(&source, false);
        let plan = o
            .prepare_compound_with_reads_v3(&edit, &req, &mut r)
            .unwrap();
        o.commit(plan, &mut r).unwrap();
        assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "a");
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::ReadDependencyAdvice,
                Boundary::ReadDependencyPreparation,
                Boundary::ReadDependencyValidation,
            ] {
                assert!(records.iter().any(|event| event.boundary == boundary));
            }
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
    }
}
