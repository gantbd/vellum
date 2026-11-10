use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::Limits;

fn setup() -> (SourceCoordinator, ProviderPort, DocumentVersion, Recorder) {
    let mut recorder = Recorder::disabled();
    let mut owner = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: None,
        history: None,
        search: None,
        annotations: Some(AnnotationLimits {
            max_issued: 16,
            max_records: 16,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 8,
            max_snapshot_bytes: 16384,
        }),
        resources: Some(ResourceLimits {
            max_providers: 4,
            max_live_tasks: 16,
            max_issued_tasks: 32,
            max_queued_per_destination: 4,
            max_outstanding: 4,
            max_blob_bytes: 4096,
            max_key_bytes: 64,
            max_validator_bytes: 64,
            max_encoders: 4,
            max_live_artifacts: 16,
            max_issued_artifacts: 64,
            max_held_artifact_bytes: 32768,
            max_future_baseline_bytes: 65536,
            max_live_reads: 8,
            max_issued_reads: 32,
            max_read_chunks: 16,
            max_held_read_bytes: 1048576,
            max_restorers: 4,
            max_restoration_bytes: 8192,
        }),
    });
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner
        .load_resource(
            port.read_result(b"source", b"s0", b"text").unwrap(),
            &mut recorder,
        )
        .unwrap();
    (owner, port, source, recorder)
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}
fn bind(
    owner: &mut SourceCoordinator,
    port: &ProviderPort,
    source: &DocumentVersion,
    recorder: &mut Recorder,
) {
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            recorder,
        )
        .unwrap();
    let evidence = port
        .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    assert!(
        owner
            .activate_annotation_binding(plan, Some(evidence), recorder)
            .unwrap()
    );
}
fn evidence(
    owner: &SourceCoordinator,
    port: &ProviderPort,
    source: &DocumentVersion,
    recorder: &mut Recorder,
    relation: ResourceRelation,
) -> OrdinaryPairEvidence {
    let check = owner
        .ordinary_pair_check(&source.document, recorder)
        .unwrap()
        .unwrap();
    port.attest_ordinary_pair(&check, relation).unwrap()
}
#[test]
fn proposed_pair_needs_known_distinction_and_same_identity_cannot_be_blessed_distinct() {
    let (mut owner, port, source, mut recorder) = setup();
    for relation in [
        None,
        Some(ResourceRelation::Unknown),
        Some(ResourceRelation::Equivalent),
    ] {
        let plan = owner
            .prepare_annotation_binding(
                &source.document,
                port.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut recorder,
            )
            .unwrap();
        let evidence = relation.map(|r| port.attest_ordinary_pair(plan.pair_check(), r).unwrap());
        let before = owner.resource_usage();
        assert!(
            owner
                .activate_annotation_binding(plan, evidence, &mut recorder)
                .is_err()
        );
        assert_eq!(owner.resource_usage(), before);
    }
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"source", b"s0", b"text").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let evidence = port
        .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    assert_eq!(
        owner.activate_annotation_binding(plan, Some(evidence), &mut recorder),
        Err(ResourceError::Conflict)
    );
}
#[test]
fn old_absent_pair_plans_cannot_both_publish_and_annotation_owned_resources_cannot_load_as_source()
{
    let (mut owner, port, source, mut recorder) = setup();
    let first = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let second = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"other", b"b0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let first_evidence = port
        .attest_ordinary_pair(first.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    let second_evidence = port
        .attest_ordinary_pair(second.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    owner
        .activate_annotation_binding(first, Some(first_evidence), &mut recorder)
        .unwrap();
    assert_eq!(
        owner.activate_annotation_binding(second, Some(second_evidence), &mut recorder),
        Err(ResourceError::Stale)
    );
    assert_eq!(
        owner.load_resource(
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder
        ),
        Err(ResourceError::Conflict)
    );
    assert_eq!(
        owner.dispose(&source.document, &mut recorder),
        Err(CoordinatorError::Resource(ResourceError::Retained))
    );
}
#[test]
fn source_admission_dispatch_and_baseline_each_need_current_exact_pair_evidence() {
    let (mut owner, port, source, mut recorder) = setup();
    let before_pair = owner
        .queue_pinned_save(&source, requirements(), &mut recorder)
        .unwrap();
    bind(&mut owner, &port, &source, &mut recorder);
    let unchanged = owner.resource_usage();
    assert_eq!(
        owner.queue_pinned_save(&source, requirements(), &mut recorder),
        Err(ResourceError::MissingEvidence)
    );
    assert_eq!(owner.resource_usage(), unchanged);
    assert!(matches!(
        owner.dispatch_save(
            &before_pair,
            port.read_result(b"source", b"s0", b"text").unwrap(),
            &mut recorder
        ),
        Err(ResourceError::MissingEvidence)
    ));
    let proof = evidence(
        &owner,
        &port,
        &source,
        &mut recorder,
        ResourceRelation::Distinct,
    );
    let dispatch = owner
        .dispatch_save_with_pair(
            &before_pair,
            port.read_result(b"source", b"s0", b"text").unwrap(),
            proof,
            &mut recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                4,
                Synchronization::NotRequested,
                Some(b"s1"),
            )
            .unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(
        owner.publish_save_baseline(&before_pair, &mut recorder),
        Err(ResourceError::MissingEvidence)
    );
    let proof = evidence(
        &owner,
        &port,
        &source,
        &mut recorder,
        ResourceRelation::Unknown,
    );
    assert_eq!(
        owner.publish_save_baseline_with_pair(&before_pair, proof, &mut recorder),
        Err(ResourceError::MissingEvidence)
    );
    let proof = evidence(
        &owner,
        &port,
        &source,
        &mut recorder,
        ResourceRelation::Distinct,
    );
    owner
        .publish_save_baseline_with_pair(&before_pair, proof, &mut recorder)
        .unwrap();
    let proof = evidence(
        &owner,
        &port,
        &source,
        &mut recorder,
        ResourceRelation::Distinct,
    );
    owner
        .queue_pinned_save_with_pair(&source, requirements(), proof, &mut recorder)
        .unwrap();
}
#[test]
fn evidence_for_a_different_owner_or_check_cannot_be_transplanted() {
    let (mut owner, port, source, mut recorder) = setup();
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let other_plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"other", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let proof = port
        .attest_ordinary_pair(other_plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    assert_eq!(
        owner.activate_annotation_binding(plan, Some(proof), &mut recorder),
        Err(ResourceError::Stale)
    );
    let (_, foreign_port, _, _) = setup();
    assert!(matches!(
        foreign_port.attest_ordinary_pair(other_plan.pair_check(), ResourceRelation::Distinct),
        Err(ResourceError::Unavailable)
    ));
}

#[test]
fn unrelated_edits_survive_binding_publication_and_identical_binding_is_unchanged() {
    for capacity in [None, Some(128), Some(1)] {
        let (mut owner, port, source, _) = setup();
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let plan = owner
            .prepare_annotation_binding(
                &source.document,
                port.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut recorder,
            )
            .unwrap();
        let proof = port
            .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
            .unwrap();
        let edit = owner
            .prepare(
                &[DocumentEdits {
                    expected: source.clone(),
                    edits: vec![vellum_engine::source::Replacement {
                        start: 0,
                        end: 4,
                        expected: "text".into(),
                        inserted: "later".into(),
                    }],
                }],
                &[],
                &mut recorder,
            )
            .unwrap();
        owner.commit(edit, &mut recorder).unwrap();
        owner
            .activate_annotation_binding(plan, Some(proof), &mut recorder)
            .unwrap();
        assert_eq!(
            owner
                .read(&source.document, &mut recorder)
                .unwrap()
                .source
                .text(),
            "later"
        );
        let generation = owner.generation();
        let usage = owner.resource_usage();
        let identical = owner
            .prepare_annotation_binding(
                &source.document,
                port.read_result(b"comments", b"a1", b"new external bytes")
                    .unwrap(),
                &mut recorder,
            )
            .unwrap();
        let proof = port
            .attest_ordinary_pair(identical.pair_check(), ResourceRelation::Distinct)
            .unwrap();
        assert!(
            !owner
                .activate_annotation_binding(identical, Some(proof), &mut recorder)
                .unwrap()
        );
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.resource_usage(), usage);
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}

#[test]
fn absent_source_is_explicit_and_global_typed_ownership_prevents_role_or_document_theft() {
    let (mut owner, port, source, mut recorder) = setup();
    let unbound = owner.create(b"notes", &mut recorder).unwrap();
    let plan = owner
        .prepare_annotation_binding(
            &unbound.document,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert!(
        owner
            .activate_annotation_binding(plan, None, &mut recorder)
            .unwrap()
    );
    assert!(
        owner
            .ordinary_pair_check(&unbound.document, &mut recorder)
            .unwrap()
            .is_none()
    );
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let proof = port
        .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    assert_eq!(
        owner.activate_annotation_binding(plan, Some(proof), &mut recorder),
        Err(ResourceError::Conflict)
    );
    let other = owner.create(b"other", &mut recorder).unwrap();
    let plan = owner
        .prepare_annotation_binding(
            &other.document,
            port.read_result(b"source", b"s0", b"text").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(
        owner.activate_annotation_binding(plan, None, &mut recorder),
        Err(ResourceError::Conflict)
    );
}

#[test]
fn first_comparison_profile_refuses_other_registrations_even_in_a_shared_identity_domain() {
    let (mut owner, port, source, mut recorder) = setup();
    let other = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            Some(&port.identity_domain()),
            &mut recorder,
        )
        .unwrap();
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            other.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert!(matches!(
        other.attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct),
        Err(ResourceError::Unsupported)
    ));
    assert!(matches!(
        port.attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct),
        Err(ResourceError::Unsupported)
    ));
    assert_eq!(
        owner.activate_annotation_binding(plan, None, &mut recorder),
        Err(ResourceError::MissingEvidence)
    );
}

#[test]
fn pending_export_and_actual_delivery_leases_block_binding_but_settled_history_does_not() {
    for launch in [false, true] {
        let (mut owner, port, source, mut recorder) = setup();
        let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
        let set = owner
            .annotation_set_version(&source.document, &mut recorder)
            .unwrap();
        let snapshot = owner
            .capture_annotations(&source, set, &mut recorder)
            .unwrap();
        let encoded = encoder.seal(snapshot, b"{}".to_vec()).unwrap();
        let artifact = owner
            .retain_annotation_artifact(encoded, &mut recorder)
            .unwrap();
        let task = owner
            .queue_annotation_export(
                &artifact,
                port.read_result(b"comments", b"a0", b"{}").unwrap(),
                requirements(),
                &mut recorder,
            )
            .unwrap();
        let plan = owner
            .prepare_annotation_binding(
                &source.document,
                port.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut recorder,
            )
            .unwrap();
        let proof = port
            .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
            .unwrap();
        assert_eq!(
            owner.activate_annotation_binding(plan, Some(proof), &mut recorder),
            Err(ResourceError::Retained)
        );
        if launch {
            let dispatch = owner
                .dispatch_save(
                    &task,
                    port.read_result(b"comments", b"a0", b"{}").unwrap(),
                    &mut recorder,
                )
                .unwrap();
            for fact in [
                port.result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
                port.no_future_effects(&dispatch).unwrap(),
                port.quiescence(&dispatch).unwrap(),
            ] {
                owner.record_save_evidence(fact, &mut recorder).unwrap();
            }
            let plan = owner
                .prepare_annotation_binding(
                    &source.document,
                    port.read_result(b"comments", b"a0", b"{}").unwrap(),
                    &mut recorder,
                )
                .unwrap();
            let proof = port
                .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
                .unwrap();
            assert_eq!(
                owner.activate_annotation_binding(plan, Some(proof), &mut recorder),
                Err(ResourceError::Retained)
            );
            drop(dispatch);
        } else {
            owner.stop_save(&task, &mut recorder).unwrap();
        }
        bind(&mut owner, &port, &source, &mut recorder);
        assert_eq!(
            owner
                .save_outcome(&task, &mut recorder)
                .unwrap()
                .binding_generation,
            None
        );
    }
}

#[test]
fn pair_plans_and_proofs_do_not_debug_disclose_resource_keys_or_external_content() {
    let (owner, port, source, mut recorder) = setup();
    let plan = owner
        .prepare_annotation_binding(
            &source.document,
            port.read_result(
                b"PRIVATE_ANNOTATION_PATH",
                b"PRIVATE_TOKEN",
                b"PRIVATE_EXTERNAL_COMMENT",
            )
            .unwrap(),
            &mut recorder,
        )
        .unwrap();
    let proof = port
        .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    let debug = format!("{plan:?} {proof:?}");
    for secret in [
        "PRIVATE_ANNOTATION_PATH",
        "PRIVATE_TOKEN",
        "PRIVATE_EXTERNAL_COMMENT",
    ] {
        assert!(!debug.contains(secret));
    }
}
