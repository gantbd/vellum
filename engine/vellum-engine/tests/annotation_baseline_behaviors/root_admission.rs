use super::*;

fn root(f: &mut Fixture) -> usize {
    f.owner
        .semantic_root_usage(&mut f.recorder)
        .unwrap()
        .total_bytes
}
fn worst_case() -> usize {
    let mut f = Fixture::new();
    f.bind();
    let id = f.ordinary(b"baseline").unwrap();
    f.written(&id, Some(&[b'v'; 64]));
    f.publish(&id, 65536).unwrap();
    root(&mut f)
}
#[test]
fn annotation_binding_activation_checks_its_actual_record_and_key_before_installation() {
    let mut probe = Fixture::new();
    probe.bind();
    let exact = root(&mut probe);
    for delta in [-1isize, 0, 1] {
        let mut f = Fixture::new();
        f.owner
            .configure_semantic_root_budget(
                exact.checked_add_signed(delta).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let plan = f
            .owner
            .prepare_annotation_binding(
                &f.source.document,
                f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let proof = f
            .provider
            .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
            .unwrap();
        let before = root(&mut f);
        let generation = f.owner.generation();
        let result = f
            .owner
            .activate_annotation_binding(plan, Some(proof), &mut f.recorder);
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(root(&mut f), before);
            assert_eq!(f.owner.generation(), generation);
        } else {
            assert_eq!(result, Ok(true));
            assert_eq!(root(&mut f), exact);
        }
    }
}
#[test]
fn ordinary_save_reserves_actual_future_root_headroom_before_spending_task_or_artifact_identity() {
    let exact = worst_case();
    for delta in [-1isize, 0, 1] {
        let mut f = Fixture::new();
        f.bind();
        f.owner
            .configure_semantic_root_budget(
                exact.checked_add_signed(delta).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let usage = f.owner.resource_usage();
        let generation = f.owner.generation();
        let before = root(&mut f);
        let result = f.ordinary(b"baseline");
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(f.owner.resource_usage(), usage);
        } else {
            result.unwrap();
        }
        assert_eq!(root(&mut f), before);
        assert_eq!(f.owner.generation(), generation);
    }
}
#[test]
fn pending_root_slot_promise_blocks_competing_growth_and_uses_maximum_not_sum_for_one_document() {
    let mut f = Fixture::new();
    f.bind();
    f.owner
        .configure_semantic_root_budget(worst_case(), &mut f.recorder)
        .unwrap();
    let one = f.ordinary(b"baseline").unwrap();
    let two = f.ordinary(b"baseline").unwrap();
    let before = root(&mut f);
    let usage = f.owner.resource_usage();
    let current = f
        .owner
        .read(&f.source.document, &mut f.recorder)
        .unwrap()
        .version;
    let p = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: current,
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "x".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner.commit(p, &mut f.recorder),
        Err(CoordinatorError::Limit)
    );
    assert_eq!(root(&mut f), before);
    assert_eq!(f.owner.resource_usage(), usage);
    assert_eq!(f.owner.generation(), generation);
    f.written(&one, Some(&[b'v'; 64]));
    assert_eq!(f.publish(&one, 65536), Ok(true));
    assert_eq!(root(&mut f), worst_case());
    assert_eq!(
        f.owner
            .save_outcome(&two, &mut f.recorder)
            .unwrap()
            .baseline_capacity,
        Some(BaselineCapacityState::Pending)
    );
}

#[test]
fn explicit_export_baseline_installation_checks_capacity_without_inventing_a_reservation() {
    fn ready() -> (Fixture, SaveId) {
        let mut f = Fixture::new();
        f.bind();
        let artifact = f.retained(b"baseline");
        let task = f.queue(&artifact, true);
        f.written(&task, Some(b"v1"));
        (f, task)
    }
    let (mut probe, task) = ready();
    probe.publish(&task, 65536).unwrap();
    let exact = root(&mut probe);
    for delta in [-1isize, 0, 1] {
        let (mut f, task) = ready();
        f.owner
            .configure_semantic_root_budget(
                exact.checked_add_signed(delta).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let before = root(&mut f);
        let generation = f.owner.generation();
        let result = f.publish(&task, 65536);
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(root(&mut f), before);
            assert!(f.fact().is_none());
        } else {
            assert_eq!(result, Ok(true));
            assert_eq!(root(&mut f), exact);
        }
        assert_eq!(
            f.owner
                .save_outcome(&task, &mut f.recorder)
                .unwrap()
                .baseline_capacity,
            None
        );
    }
}

#[test]
fn capacity_gauges_separate_current_root_from_future_demand_and_late_configuration_preserves_existing_holds()
 {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let mut f = Fixture::new();
        f.bind();
        f.ordinary(b"baseline").unwrap();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let usage = f.owner.resource_usage();
        let generation = f.owner.generation();
        let demand = f
            .owner
            .semantic_root_capacity_usage(&mut f.recorder)
            .unwrap();
        assert_eq!(demand.current_root_bytes, root(&mut f));
        assert_eq!(demand.admission_bytes, worst_case());
        assert!(demand.pending_baseline_growth_bytes > 0);
        assert_eq!(
            demand.current_root_bytes + demand.pending_baseline_growth_bytes,
            demand.admission_bytes
        );
        assert_eq!(
            f.owner
                .configure_semantic_root_budget(demand.admission_bytes - 1, &mut f.recorder),
            Err(RootAccountingErrorV1::Limit)
        );
        assert_eq!(f.owner.semantic_root_byte_ceiling(), None);
        assert_eq!(
            f.owner
                .configure_semantic_root_budget(demand.admission_bytes, &mut f.recorder),
            Ok(true)
        );
        assert_eq!(f.owner.resource_usage(), usage);
        assert_eq!(f.owner.generation(), generation);
        if capacity == Some(128) {
            assert!(
                std::iter::from_fn(|| f.recorder.pop_front())
                    .any(|e| e.boundary == Boundary::SemanticRootCapacityInspection
                        && e.disposition == Disposition::Completed)
            );
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
        if capacity.is_none() {
            assert_eq!(f.recorder.queued(), 0);
        }
    }
}

#[test]
fn stopped_unknown_effect_keeps_root_headroom_until_actual_discharge_then_competing_work_can_proceed()
 {
    let mut f = Fixture::new();
    f.bind();
    f.owner
        .configure_semantic_root_budget(worst_case(), &mut f.recorder)
        .unwrap();
    let id = f.ordinary(b"baseline").unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &id,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    f.owner.stop_save(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let before = f
        .owner
        .semantic_root_capacity_usage(&mut f.recorder)
        .unwrap();
    let p = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: f.source.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "x".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.commit(p, &mut f.recorder),
        Err(CoordinatorError::Limit)
    );
    assert_eq!(
        f.owner
            .semantic_root_capacity_usage(&mut f.recorder)
            .unwrap(),
        before
    );
    for evidence in [
        f.provider
            .result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
        f.provider.no_future_effects(&dispatch).unwrap(),
        f.provider.quiescence(&dispatch).unwrap(),
    ] {
        f.owner
            .record_save_evidence(evidence, &mut f.recorder)
            .unwrap();
    }
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&id, &mut f.recorder),
        Ok(BaselineCapacityState::NotWritten)
    );
    assert_eq!(
        f.owner
            .semantic_root_capacity_usage(&mut f.recorder)
            .unwrap()
            .pending_baseline_growth_bytes,
        0
    );
    let p = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: f.source.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "x".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(p, &mut f.recorder).unwrap();
}

fn second_document(f: &mut Fixture) -> DocumentVersion {
    let source = f
        .owner
        .load_resource(
            f.provider.read_result(b"source2", b"s0", b"text").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let plan = f
        .owner
        .prepare_annotation_binding(
            &source.document,
            f.provider.read_result(b"comments2", b"a0", b"{}").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let proof = f
        .provider
        .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    f.owner
        .activate_annotation_binding(plan, Some(proof), &mut f.recorder)
        .unwrap();
    source
}
fn second_save(f: &mut Fixture, source: &DocumentVersion) -> Result<SaveId, ResourceError> {
    let snapshot = f
        .owner
        .capture_annotations(source, 0, &mut f.recorder)
        .unwrap();
    let encoded = f.encoder.seal(snapshot, b"baseline".to_vec()).unwrap();
    let check = f
        .owner
        .ordinary_pair_check(&source.document, &mut f.recorder)
        .unwrap()
        .unwrap();
    let pair = f
        .provider
        .attest_ordinary_pair(&check, ResourceRelation::Distinct)
        .unwrap();
    f.owner.queue_annotation_save(
        encoded,
        f.provider.read_result(b"comments2", b"a0", b"{}").unwrap(),
        SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Synchronized,
        },
        Some(pair),
        &mut f.recorder,
    )
}
#[test]
fn distinct_documents_add_root_slot_demands_while_every_hold_keeps_its_own_retention_charge() {
    let mut probe = Fixture::new();
    probe.bind();
    let second = second_document(&mut probe);
    let first = probe.ordinary(b"baseline").unwrap();
    let first_demand = probe
        .owner
        .semantic_root_capacity_usage(&mut probe.recorder)
        .unwrap();
    second_save(&mut probe, &second).unwrap();
    let both = probe
        .owner
        .semantic_root_capacity_usage(&mut probe.recorder)
        .unwrap();
    probe.owner.stop_save(&first, &mut probe.recorder).unwrap();
    let second_demand = probe
        .owner
        .semantic_root_capacity_usage(&mut probe.recorder)
        .unwrap();
    assert_eq!(
        both.pending_baseline_growth_bytes,
        first_demand.pending_baseline_growth_bytes + second_demand.pending_baseline_growth_bytes
    );
    assert!(
        first_demand.pending_baseline_growth_bytes > 0
            && second_demand.pending_baseline_growth_bytes > 0
    );
    for delta in [-1isize, 0] {
        let mut f = Fixture::new();
        f.bind();
        let second = second_document(&mut f);
        f.owner
            .configure_semantic_root_budget(
                both.admission_bytes.checked_add_signed(delta).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        f.ordinary(b"baseline").unwrap();
        let usage = f.owner.resource_usage();
        let result = second_save(&mut f, &second);
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(f.owner.resource_usage(), usage);
        } else {
            result.unwrap();
        }
    }
}

#[test]
fn settings_preparation_preserves_pending_baseline_headroom_separately_from_configuration_limits() {
    use vellum_engine::settings::*;
    fn id(name: &str) -> SettingIdentityV1 {
        SettingIdentityV1 {
            authority: "engine".into(),
            id: name.into(),
            incarnation: 1,
        }
    }
    let mut f = Fixture::new();
    f.bind();
    let definition = SettingsDefinitionV1 {
        schemas: vec![SettingSchemaV1 {
            key: "flag".into(),
            value_type: SettingTypeV1::Boolean,
            default: SettingValueV1::Boolean(false),
            domain: SettingDomainV1::Analysis,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: false,
        }],
        restrictions: vec![],
        layers: vec![SettingLayerV1 {
            identity: id("host"),
            scope: SettingScopeV1::Host,
            generation: 1,
            values: vec![],
        }],
        targets: vec![SettingTargetV1 {
            identity: id("target"),
            layers: vec![id("host")],
        }],
    };
    let compiled = compile_settings_v1(
        &definition,
        "engine",
        SettingsLimitsV1 {
            max_keys: 8,
            max_layers: 8,
            max_targets: 8,
            max_identifier_bytes: 64,
            max_schema_nodes: 64,
            max_schema_depth: 16,
            max_value_nodes: 64,
            max_value_depth: 16,
            max_state_bytes: 65536,
            max_work: 65536,
        },
        &mut f.recorder,
    )
    .unwrap();
    f.owner
        .install_initial_settings(compiled, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_settings_activation(
            SettingsActivationLimitsV1 {
                max_changes: 8,
                max_request_nodes: 64,
                max_request_depth: 16,
                max_request_bytes: 16384,
                max_plan_bytes: 65536,
                max_work: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    let save = f.ordinary(&[b'x'; 512]).unwrap();
    let demand = f
        .owner
        .semantic_root_capacity_usage(&mut f.recorder)
        .unwrap();
    f.owner
        .configure_semantic_root_budget(demand.admission_bytes, &mut f.recorder)
        .unwrap();
    let q = [SettingChangeV1 {
        layer: id("host"),
        expected_generation: 1,
        key: "flag".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
    }];
    let generation = f.owner.generation();
    assert!(matches!(
        f.owner.prepare_settings_change(&q, &mut f.recorder),
        Err(SettingsError::Limit)
    ));
    assert_eq!(f.owner.generation(), generation);
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    let p = f
        .owner
        .prepare_settings_change(&q, &mut f.recorder)
        .unwrap();
    f.owner.commit_settings_change(p, &mut f.recorder).unwrap();
    assert!(root(&mut f) <= demand.admission_bytes);
}
