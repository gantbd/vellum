use super::*;
#[test]
fn annotation_baseline_service_timings_preserve_comparison_and_budgets() {
    use crate::instrumentation::NativeTimingProfileV1;
    let mut reference = None;
    for mode in 0..4 {
        let (mut o, p, id, mut setup) = fixture();
        let document = o.resource_task(&id).unwrap().captured.clone();
        let dispatch = o
            .dispatch_save(
                &id,
                p.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut setup,
            )
            .unwrap();
        o.record_save_evidence(
            p.result(
                &dispatch,
                Application::Written,
                2,
                Synchronization::NotRequested,
                Some(b"a1"),
            )
            .unwrap(),
            &mut setup,
        )
        .unwrap();
        let mut r = Recorder::try_with_capacity([0, 64, 64, 1][mode])
            .unwrap()
            .with_native_timing(if mode >= 2 {
                NativeTimingProfileV1::ProcessMonotonicV1
            } else {
                NativeTimingProfileV1::Off
            });
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Query,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        assert!(
            o.publish_annotation_baseline(&id, None, 65536, &mut r)
                .unwrap()
        );
        let baseline = o
            .annotation_baseline(&document.document, None, &mut r)
            .unwrap()
            .unwrap();
        assert_eq!(baseline.source, document);
        // Compare under the baseline's actual encoder incarnation, not a newly
        // registered encoder with a superficially similar format.
        let encoder = AnnotationEncoderPort {
            registration: o.resources.encoders[0].clone(),
        };
        let snapshot = o.capture_annotations(&document, 0, &mut setup).unwrap();
        let current = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
        assert_eq!(
            o.compare_annotation_baseline(&document.document, &current, None, 4, &mut r),
            Ok(ContentComparison::Unchanged)
        );
        assert_eq!(
            o.compare_annotation_baseline(&document.document, &current, None, 3, &mut r),
            Err(ResourceError::Limit)
        );
        if mode == 1 || mode == 2 {
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for (boundary, dispositions) in [
                (
                    Boundary::AnnotationBaselinePublication,
                    vec![Disposition::Completed],
                ),
                (
                    Boundary::AnnotationBaselineQuery,
                    vec![Disposition::Completed],
                ),
                (
                    Boundary::AnnotationComparison,
                    vec![Disposition::Completed, Disposition::Refused],
                ),
            ] {
                let selected: Vec<_> = records.iter().filter(|e| e.boundary == boundary).collect();
                assert_eq!(selected.len(), dispositions.len());
                for (record, disposition) in selected.into_iter().zip(dispositions) {
                    assert_eq!(record.disposition, disposition);
                    assert_eq!(record.service_time.is_some(), mode == 2, "{boundary:?}");
                }
            }
        }
        let summary = (o.resource_usage(), o.generation(), o.retention_usage());
        if let Some(expected) = &reference {
            assert_eq!(&summary, expected);
        } else {
            reference = Some(summary);
        }
        assert_eq!(r.dropped() > 0, mode == 3);
        if mode == 0 {
            assert_eq!(r.queued(), 0);
        }
    }
}

fn fixture() -> (SourceCoordinator, ProviderPort, SaveId, Recorder) {
    let mut recorder = Recorder::disabled();
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 2,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 128,
            max_edits: 2,
            max_edit_payload_bytes: 128,
        },
    });
    owner.resources = ResourceState::new(ResourceLimits {
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
    owner.annotations.limits = Some(AnnotationLimits {
        max_issued: 4,
        max_records: 4,
        max_body_bytes: 128,
        max_retained_body_bytes: 512,
        max_mapping_steps: 128,
        max_live_snapshots: 4,
        max_snapshot_bytes: 4096,
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
    let doc = owner.create(b"notes", &mut recorder).unwrap();
    let plan = owner
        .prepare_annotation_binding(
            &doc.document,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .activate_annotation_binding(plan, None, &mut recorder)
        .unwrap();
    let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
    let snapshot = owner.capture_annotations(&doc, 0, &mut recorder).unwrap();
    let encoded = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
    let artifact = owner
        .retain_annotation_artifact(encoded, &mut recorder)
        .unwrap();
    let task = owner
        .queue_bound_annotation_export(
            &artifact,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    (owner, port, task, recorder)
}

#[test]
fn admitted_publication_credit_is_preserved_by_other_native_publication_routes() {
    let (mut owner, port, prior, mut recorder) = fixture();
    let document = owner.resource_task(&prior).unwrap().captured.clone();
    owner.stop_save(&prior, &mut recorder).unwrap();
    let source = owner
        .load_resource(
            port.read_result(b"source", b"s0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let source_task = owner
        .queue_pinned_save(
            &source,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut recorder,
        )
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &source_task,
            port.read_result(b"source", b"s0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"s1"),
            )
            .unwrap(),
            &mut recorder,
        )
        .unwrap();
    // Both admitted source and annotation producers now own a publication unit.
    owner.generation = u64::MAX - 3;
    let snapshot = owner
        .capture_annotations(&document, 0, &mut recorder)
        .unwrap();
    let encoder = AnnotationEncoderPort {
        registration: owner.resources.encoders[0].clone(),
    };
    let pending = owner
        .queue_annotation_save(
            encoder.seal(snapshot, b"[]".to_vec()).unwrap(),
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let other = owner.create(b"other", &mut recorder).unwrap();
    assert_eq!(owner.generation, u64::MAX - 2);
    assert_eq!(
        owner.create(b"new", &mut recorder),
        Err(CoordinatorError::VersionExhausted)
    );
    assert_eq!(
        owner.set_read_only(&document.document, true, &mut recorder),
        Err(CoordinatorError::VersionExhausted)
    );
    assert_eq!(
        owner.set_annotations_read_only(&document.document, true, &mut recorder),
        Err(CoordinatorError::VersionExhausted)
    );
    assert_eq!(
        owner.dispose(&other.document, &mut recorder),
        Err(CoordinatorError::VersionExhausted)
    );
    let plan = owner
        .prepare_annotation_binding(
            &other.document,
            port.read_result(b"other-comments", b"v0", b"{}").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert_eq!(
        owner.activate_annotation_binding(plan, None, &mut recorder),
        Err(ResourceError::VersionExhausted)
    );
    owner
        .publish_save_baseline(&source_task, &mut recorder)
        .unwrap();
    assert_eq!(owner.generation, u64::MAX - 1);
    assert_eq!(owner.resource_usage().pending_baseline_publications, 1);
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: document.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 5,
                    expected: "notes".into(),
                    inserted: "different".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    assert_eq!(
        owner.commit(edit, &mut recorder),
        Err(CoordinatorError::VersionExhausted)
    );
    let noop = owner
        .prepare(
            &[DocumentEdits {
                expected: document.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 5,
                    expected: "notes".into(),
                    inserted: "notes".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    owner.commit(noop, &mut recorder).unwrap();
    drop(written(&mut owner, &port, &pending, &mut recorder));
    assert!(
        owner
            .install_annotation_save_baseline(&pending, None, &mut recorder)
            .unwrap()
    );
    assert_eq!(owner.generation, u64::MAX);
    assert_eq!(owner.resource_usage().pending_baseline_publications, 0);
    assert!(
        !owner
            .install_annotation_save_baseline(&pending, None, &mut recorder)
            .unwrap()
    );
}
fn written(
    owner: &mut SourceCoordinator,
    port: &ProviderPort,
    task: &SaveId,
    recorder: &mut Recorder,
) -> SaveDispatch {
    let dispatch = owner
        .dispatch_save(
            task,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                2,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
            recorder,
        )
        .unwrap();
    owner
        .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), recorder)
        .unwrap();
    owner
        .record_save_evidence(port.quiescence(&dispatch).unwrap(), recorder)
        .unwrap();
    dispatch
}

fn ordinary(
    owner: &mut SourceCoordinator,
    port: &ProviderPort,
    prior: &SaveId,
    recorder: &mut Recorder,
) -> Result<SaveId, ResourceError> {
    let document = owner
        .resource_task(prior)
        .unwrap()
        .captured
        .document
        .clone();
    let source = owner.read(&document, recorder).unwrap().version;
    let set = owner.annotation_set_version(&document, recorder).unwrap();
    let snapshot = owner.capture_annotations(&source, set, recorder).unwrap();
    let encoder = AnnotationEncoderPort {
        registration: owner.resources.encoders[0].clone(),
    };
    owner.queue_annotation_save(
        encoder.seal(snapshot, b"[]".to_vec()).unwrap(),
        port.read_result(b"comments", b"a0", b"{}").unwrap(),
        SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        },
        None,
        recorder,
    )
}

#[test]
fn two_holds_for_one_document_reserve_two_counter_units_not_one() {
    let (mut owner, port, prior, mut recorder) = fixture();
    owner.stop_save(&prior, &mut recorder).unwrap();
    owner.generation = u64::MAX - 2;
    let first = ordinary(&mut owner, &port, &prior, &mut recorder).unwrap();
    let second = ordinary(&mut owner, &port, &prior, &mut recorder).unwrap();
    let usage = owner.resource_usage();
    assert_eq!(usage.pending_baseline_publications, 2);
    assert_eq!(
        ordinary(&mut owner, &port, &prior, &mut recorder),
        Err(ResourceError::VersionExhausted)
    );
    assert_eq!(owner.resource_usage(), usage);
    let document = owner
        .resource_task(&prior)
        .unwrap()
        .captured
        .document
        .clone();
    assert_eq!(
        owner.set_read_only(&document, true, &mut recorder),
        Err(CoordinatorError::VersionExhausted)
    );
    owner.stop_save(&first, &mut recorder).unwrap();
    owner.set_read_only(&document, true, &mut recorder).unwrap();
    assert_eq!(owner.generation, u64::MAX - 1);
    drop(written(&mut owner, &port, &second, &mut recorder));
    assert!(
        owner
            .install_annotation_save_baseline(&second, None, &mut recorder)
            .unwrap()
    );
    assert_eq!(owner.generation, u64::MAX);
}

#[test]
fn protected_nonapplication_cleanup_after_native_fault_preserves_issued_writer_evidence() {
    let (mut owner, port, prior, mut recorder) = fixture();
    owner.stop_save(&prior, &mut recorder).unwrap();
    let task = ordinary(&mut owner, &port, &prior, &mut recorder).unwrap();
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
    owner.faulted = true;
    owner.stop_annotation_save_admission(&mut recorder);
    let before = owner.resource_usage();
    let generation = owner.generation;
    assert_eq!(
        owner.release_annotation_save_capacity(&task, &mut recorder),
        Ok(BaselineCapacityState::NotWritten)
    );
    let after = owner.resource_usage();
    assert_eq!(after.pending_baseline_publications, 0);
    assert_eq!(
        after.issued_artifacts_lifetime,
        before.issued_artifacts_lifetime
    );
    assert_eq!(after.held_artifact_bytes, before.held_artifact_bytes);
    assert_eq!(after.retained, before.retained);
    assert_eq!(owner.generation, generation);
    assert!(owner.faulted);
    assert_eq!(
        owner.retire_save(&task, &mut recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    owner.retire_save(&task, &mut recorder).unwrap();
}

#[test]
fn a_forged_newer_baseline_serial_cannot_discharge_an_older_capacity_promise() {
    let (mut owner, port, prior, mut recorder) = fixture();
    owner.stop_save(&prior, &mut recorder).unwrap();
    let first = ordinary(&mut owner, &port, &prior, &mut recorder).unwrap();
    drop(written(&mut owner, &port, &first, &mut recorder));
    let second = ordinary(&mut owner, &port, &prior, &mut recorder).unwrap();
    drop(written(&mut owner, &port, &second, &mut recorder));
    owner
        .install_annotation_save_baseline(&second, None, &mut recorder)
        .unwrap();
    owner.resources.annotation_bindings[0]
        .baseline
        .as_mut()
        .unwrap()
        .written_mut()
        .unwrap()
        .fact
        .dispatch_serial += 1;
    let usage = owner.resource_usage();
    let generation = owner.generation;
    assert_eq!(
        owner.release_annotation_save_capacity(&first, &mut recorder),
        Err(ResourceError::Faulted)
    );
    assert!(owner.faulted);
    assert_eq!(owner.resource_usage(), usage);
    assert_eq!(owner.generation, generation);
}
#[test]
fn exact_native_baseline_bound_and_counter_exhaustion_do_not_partially_adopt() {
    let (mut owner, port, task, mut recorder) = fixture();
    drop(written(&mut owner, &port, &task, &mut recorder));
    let bound = owner
        .resource_task(&task)
        .unwrap()
        .annotation_baseline(&task)
        .unwrap()
        .retained_size()
        .unwrap();
    assert_eq!(
        owner.publish_annotation_baseline(&task, None, bound - 1, &mut recorder),
        Err(ResourceError::Limit)
    );
    owner.generation = u64::MAX;
    assert_eq!(
        owner.publish_annotation_baseline(&task, None, bound, &mut recorder),
        Err(ResourceError::VersionExhausted)
    );
    assert!(owner.resources.annotation_bindings[0].baseline.is_none());
    owner.generation = 1;
    assert!(
        owner
            .publish_annotation_baseline(&task, None, bound, &mut recorder)
            .unwrap()
    );
    owner.generation = u64::MAX;
    assert!(
        !owner
            .publish_annotation_baseline(&task, None, bound, &mut recorder)
            .unwrap()
    );
    assert_eq!(owner.generation, u64::MAX);
}
#[test]
fn dispatch_serial_is_spent_only_at_launch_and_exhaustion_keeps_the_task_queued() {
    let (mut owner, port, task, mut recorder) = fixture();
    owner.resources.last_dispatch_serial = u64::MAX;
    let usage = owner.resource_usage();
    assert!(matches!(
        owner.dispatch_save(
            &task,
            port.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut recorder
        ),
        Err(ResourceError::VersionExhausted)
    ));
    assert_eq!(owner.resource_usage(), usage);
    let outcome = owner.save_outcome(&task, &mut recorder).unwrap();
    assert_eq!(outcome.phase, SavePhase::Queued);
    assert_eq!(outcome.dispatch_serial, None);
    owner.resources.last_dispatch_serial = u64::MAX - 1;
    drop(written(&mut owner, &port, &task, &mut recorder));
    assert_eq!(
        owner
            .save_outcome(&task, &mut recorder)
            .unwrap()
            .dispatch_serial,
        Some(u64::MAX)
    );
}
#[test]
fn corrupt_existing_baseline_refuses_and_fences_instead_of_repairing_it() {
    for field in 0..4 {
        let (mut owner, port, task, mut recorder) = fixture();
        let dispatch = written(&mut owner, &port, &task, &mut recorder);
        owner
            .publish_annotation_baseline(&task, None, 65536, &mut recorder)
            .unwrap();
        let fact = &mut owner.resources.annotation_bindings[0]
            .baseline
            .as_mut()
            .unwrap()
            .written_mut()
            .unwrap()
            .fact;
        match field {
            0 => fact.set_version += 1,
            1 => fact.ordinary_binding_generation += 1,
            2 => fact.source_binding_generation = Some(1),
            _ => fact.required_acknowledgment = Acknowledgment::Synchronized,
        }
        let generation = owner.generation;
        let result = if field % 2 == 0 {
            owner
                .publish_annotation_baseline(&task, None, 65536, &mut recorder)
                .map(|_| ())
        } else {
            let document = owner
                .resource_task(&task)
                .unwrap()
                .captured
                .document
                .clone();
            owner
                .annotation_baseline(&document, None, &mut recorder)
                .map(|_| ())
        };
        assert_eq!(result, Err(ResourceError::Faulted));
        assert!(owner.faulted);
        assert_eq!(owner.generation, generation);
        owner
            .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut recorder)
            .unwrap();
    }
}
