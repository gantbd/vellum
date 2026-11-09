use super::*;
pub(in crate::coordinator) fn fixture()
-> (SourceCoordinator, Recorder, DocumentVersion, ProviderPort) {
    let (mut o, mut r, _) = super::super::serialization::tests::setup();
    o.resources = ResourceState::new(ResourceLimits {
        max_providers: 2,
        max_live_tasks: 4,
        max_issued_tasks: 8,
        max_queued_per_destination: 4,
        max_outstanding: 4,
        max_blob_bytes: 4096,
        max_key_bytes: 64,
        max_validator_bytes: 64,
        max_encoders: 2,
        max_live_artifacts: 4,
        max_issued_artifacts: 8,
        max_held_artifact_bytes: 8192,
        max_future_baseline_bytes: 65536,
        max_live_reads: 8,
        max_issued_reads: 32,
        max_read_chunks: 16,
        max_held_read_bytes: 1048576,
        max_restorers: 4,
        max_restoration_bytes: 8192,
    });
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let d = o
        .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    (o, r, d, port)
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}

pub(in crate::coordinator) fn pending_root_fixture()
-> (SourceCoordinator, Recorder, SaveId, ProviderPort) {
    let (mut o, mut r, d, port) = fixture();
    o.annotations.limits = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 1024,
        max_mapping_steps: 4096,
        max_live_snapshots: 8,
        max_snapshot_bytes: 4096,
    });
    let plan = o
        .prepare_annotation_binding(
            &d.document,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            &mut r,
        )
        .unwrap();
    let pair = port
        .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
        .unwrap();
    o.activate_annotation_binding(plan, Some(pair), &mut r)
        .unwrap();
    let encoder = o.register_annotation_encoder(&mut r).unwrap();
    let snapshot = o.capture_annotations(&d, 0, &mut r).unwrap();
    let encoded = encoder.seal(snapshot, b"baseline".to_vec()).unwrap();
    let pair = o.ordinary_pair_check(&d.document, &mut r).unwrap().unwrap();
    let pair = port
        .attest_ordinary_pair(&pair, ResourceRelation::Distinct)
        .unwrap();
    let id = o
        .queue_annotation_save(
            encoded,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            requirements(),
            Some(pair),
            &mut r,
        )
        .unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    (o, r, id, port)
}

#[test]
fn corrupted_pending_root_promises_fault_admission_without_issuing_and_protected_stop_still_works()
{
    for corrupt in [false, true] {
        for birth in [false, true] {
            let (mut o, mut r, id, _) = pending_root_fixture();
            let task = o.resources.tasks[id.slot].as_mut().unwrap();
            if corrupt {
                task.annotation_association = None;
            } else {
                task.baseline_claim.as_mut().unwrap().charge = 0;
            }
            assert_eq!(
                o.semantic_root_capacity_usage(&mut r),
                Err(RootAccountingErrorV1::Faulted)
            );
            let generation = o.generation();
            let documents = o.documents.len();
            let providers = o.resources.providers.len();
            if birth {
                assert_eq!(o.create(b"new", &mut r), Err(CoordinatorError::Faulted));
            } else {
                assert!(matches!(
                    o.register_provider(
                        ProviderProfile {
                            synchronization_supported: false
                        },
                        None,
                        &mut r
                    ),
                    Err(ResourceError::Faulted)
                ));
            }
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation);
            assert_eq!(o.documents.len(), documents);
            assert_eq!(o.resources.providers.len(), providers);
            o.stop_save(&id, &mut r).unwrap();
            assert!(o.is_faulted());
            assert_eq!(
                o.resources.tasks[id.slot]
                    .as_ref()
                    .unwrap()
                    .baseline_claim
                    .as_ref()
                    .unwrap()
                    .state,
                BaselineCapacityState::NotWritten
            );
        }
    }
}

#[test]
fn consuming_a_hold_cannot_launder_an_underreserved_private_charge() {
    let (mut o, mut r, id, port) = pending_root_fixture();
    let doc = o.resource_task(&id).unwrap().captured.document.clone();
    let check = o.ordinary_pair_check(&doc, &mut r).unwrap().unwrap();
    let pair = port
        .attest_ordinary_pair(&check, ResourceRelation::Distinct)
        .unwrap();
    let dispatch = o
        .dispatch_save_with_pair(
            &id,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            pair,
            &mut r,
        )
        .unwrap();
    let evidence = port
        .result(
            &dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap();
    o.record_save_evidence(evidence, &mut r).unwrap();
    o.resources.tasks[id.slot]
        .as_mut()
        .unwrap()
        .baseline_claim
        .as_mut()
        .unwrap()
        .charge -= 1;
    let before = o.semantic_root_usage(&mut r).unwrap();
    let generation = o.generation();
    let check = o.ordinary_pair_check(&doc, &mut r).unwrap().unwrap();
    let pair = port
        .attest_ordinary_pair(&check, ResourceRelation::Distinct)
        .unwrap();
    assert_eq!(
        o.install_annotation_save_baseline(&id, Some(pair), &mut r),
        Err(ResourceError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    assert_eq!(
        o.resource_task(&id)
            .unwrap()
            .baseline_claim
            .as_ref()
            .unwrap()
            .state,
        BaselineCapacityState::Pending
    );
}
#[test]
fn invalid_current_serialization_refuses_before_save_issuance_and_faults_owner() {
    let (mut o, mut r, d, _) = fixture();
    let before = o.resource_usage();
    o.documents[d.document.slot]
        .as_mut()
        .unwrap()
        .serialization
        .generation = 0;
    assert_eq!(
        o.queue_pinned_save(&d, requirements(), &mut r),
        Err(ResourceError::Faulted)
    );
    assert_eq!(o.resource_usage(), before);
    assert!(o.is_faulted());
}
#[test]
fn capture_metadata_loss_or_drift_faults_before_dispatch_or_baseline_publication() {
    for baseline in [false, true] {
        for missing in [false, true] {
            let (mut o, mut r, d, port) = fixture();
            let job = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
            let task = o.resources.tasks[job.slot].as_mut().unwrap();
            if missing {
                task.serialization = None;
            } else {
                task.serialization.as_mut().unwrap().generation += 1;
            }
            let before = o.resource_usage();
            let result = if baseline {
                o.publish_save_baseline(&job, &mut r)
            } else {
                o.dispatch_save(&job, port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
                    .map(|_| ())
            };
            assert_eq!(result, Err(ResourceError::Faulted));
            assert!(o.is_faulted());
            assert_eq!(o.resource_usage(), before);
            assert_eq!(
                o.resources.tasks[job.slot].as_ref().unwrap().phase,
                SavePhase::Queued
            );
        }
    }
}
