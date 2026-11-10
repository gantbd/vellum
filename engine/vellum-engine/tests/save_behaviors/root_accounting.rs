use super::*;
#[test]
fn source_baseline_enrichment_checks_the_actual_candidate_before_generation_or_binding_change() {
    fn ready() -> (SourceCoordinator, vellum_engine::resource::SaveId, Recorder) {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let port = o
            .register_provider(
                ProviderProfile {
                    synchronization_supported: true,
                },
                None,
                &mut r,
            )
            .unwrap();
        let d = o
            .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
            .unwrap();
        let id = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let dispatch = o
            .dispatch_save(&id, port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
            .unwrap();
        // The original installation spends the admitted reservation. Later
        // optional token enrichment must obtain fresh root headroom.
        let tokenless = port
            .result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                None,
            )
            .unwrap();
        o.record_save_evidence(tokenless, &mut r).unwrap();
        o.publish_save_baseline(&id, &mut r).unwrap();
        let evidence = port
            .result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"longer"),
            )
            .unwrap();
        o.record_save_evidence(evidence, &mut r).unwrap();
        (o, id, r)
    }
    let (mut probe, id, mut r) = ready();
    probe.publish_save_baseline(&id, &mut r).unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    for delta in [-1isize, 0, 1] {
        let (mut o, id, mut r) = ready();
        o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
            .unwrap();
        let before = o.semantic_root_usage(&mut r).unwrap();
        let generation = o.generation();
        let state = o.save_outcome(&id, &mut r).unwrap();
        let result = o.publish_save_baseline(&id, &mut r);
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(o.generation(), generation);
            assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
            assert_eq!(o.save_outcome(&id, &mut r).unwrap(), state);
        } else {
            result.unwrap();
            assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
        }
    }
}
fn recorder(capacity: Option<usize>) -> Recorder {
    capacity.map_or_else(Recorder::disabled, |n| {
        Recorder::try_with_capacity(n).unwrap()
    })
}
fn signal(r: &mut Recorder, capacity: Option<usize>, refused: bool) {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    if capacity == Some(128) {
        assert!(std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary
            == Boundary::SemanticRootAdmission
            && e.disposition
                == if refused {
                    Disposition::Refused
                } else {
                    Disposition::Completed
                }));
    }
    if capacity == Some(1) {
        assert!(r.dropped() > 0);
    }
    if capacity.is_none() {
        assert_eq!(r.queued(), 0);
    }
}

fn registration_owner() -> SourceCoordinator {
    owner_with_annotations(Some(vellum_engine::coordinator::AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 1024,
        max_mapping_steps: 4096,
        max_live_snapshots: 8,
        max_snapshot_bytes: 4096,
    }))
}
fn register(o: &mut SourceCoordinator, kind: usize, r: &mut Recorder) -> Result<(), ResourceError> {
    match kind {
        0 => o
            .register_provider(
                ProviderProfile {
                    synchronization_supported: true,
                },
                None,
                r,
            )
            .map(|_| ()),
        1 => o.register_annotation_encoder(r).map(|_| ()),
        2 => o.register_annotation_restorer(r).map(|_| ()),
        _ => unreachable!(),
    }
}
#[test]
fn provider_encoder_and_restorer_registrations_admit_actual_registry_growth_before_issuance() {
    for kind in 0..3 {
        let mut probe = registration_owner();
        let mut r = Recorder::disabled();
        register(&mut probe, kind, &mut r).unwrap();
        let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
        for delta in [-1isize, 0, 1] {
            for capacity in [None, Some(128), Some(1)] {
                let mut r = recorder(capacity);
                let mut o = registration_owner();
                o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
                    .unwrap();
                let before = o.semantic_root_usage(&mut r).unwrap();
                let usage = o.resource_usage();
                let generation = o.generation();
                let result = register(&mut o, kind, &mut r);
                if delta < 0 {
                    assert_eq!(result, Err(ResourceError::Limit));
                    assert_eq!(o.resource_usage(), usage);
                    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
                } else {
                    result.unwrap();
                    assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
                    // Dropping the returned port does not erase the owner registry.
                    assert_eq!(register(&mut o, kind, &mut r), Err(ResourceError::Limit));
                    assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
                }
                assert_eq!(o.generation(), generation);
                assert!(!o.is_faulted());
                signal(&mut r, capacity, delta < 0);
            }
        }
    }
}

#[test]
fn loaded_source_birth_admits_source_and_binding_baseline_together_before_either_is_installed() {
    let mut probe = owner();
    let mut r = Recorder::disabled();
    let port = probe
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut r,
        )
        .unwrap();
    probe
        .load_resource(
            port.read_result(b"key", b"version", b"\xef\xbb\xbfA")
                .unwrap(),
            &mut r,
        )
        .unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    for delta in [-1isize, 0, 1] {
        for capacity in [None, Some(128), Some(1)] {
            let mut r = recorder(capacity);
            let mut o = owner();
            let port = o
                .register_provider(
                    ProviderProfile {
                        synchronization_supported: true,
                    },
                    None,
                    &mut r,
                )
                .unwrap();
            o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
                .unwrap();
            let before = o.semantic_root_usage(&mut r).unwrap();
            let usage = o.resource_usage();
            let lifecycle = o.lifecycle_usage();
            let generation = o.generation();
            let result = o.load_resource(
                port.read_result(b"key", b"version", b"\xef\xbb\xbfA")
                    .unwrap(),
                &mut r,
            );
            if delta < 0 {
                assert_eq!(result, Err(ResourceError::Limit));
                assert_eq!(o.resource_usage(), usage);
                assert_eq!(o.lifecycle_usage(), lifecycle);
                assert_eq!(o.generation(), generation);
                assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
            } else {
                let d = result.unwrap();
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
                let generation = o.generation();
                // Known aliases join unchanged, not a second birth/baseline charge.
                assert_eq!(
                    o.load_resource(
                        port.read_result(b"key", b"other", b"changed").unwrap(),
                        &mut r
                    ),
                    Ok(d)
                );
                assert_eq!(o.generation(), generation);
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
            }
            signal(&mut r, capacity, delta < 0);
        }
    }
}

#[test]
fn root_counts_actual_persistence_baseline_and_history_not_queued_save_capture() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let empty = o.semantic_root_usage(&mut r).unwrap();
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut r,
        )
        .unwrap();
    let registered = o.semantic_root_usage(&mut r).unwrap();
    assert!(registered.persistence_bytes > empty.persistence_bytes);
    let d = o
        .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    let loaded = o.semantic_root_usage(&mut r).unwrap();
    assert!(loaded.persistence_bytes > registered.persistence_bytes);
    let prepared = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "longer".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), loaded);
    o.commit(prepared, &mut r).unwrap();
    let edited = o.semantic_root_usage(&mut r).unwrap();
    assert!(edited.history_bytes > loaded.history_bytes);
    assert!(edited.documents_bytes > loaded.documents_bytes + 5); // source + frontier
    assert_eq!(edited.persistence_bytes, loaded.persistence_bytes);
    let current = o.read(&d.document, &mut r).unwrap().version;
    let task = o
        .queue_pinned_save(&current, requirements(), &mut r)
        .unwrap();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), edited);
    let dispatch = o
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), edited);
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
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), edited);
    o.publish_save_baseline(&task, &mut r).unwrap();
    let saved = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(saved.persistence_bytes - edited.persistence_bytes, 5);
    assert_eq!(saved.history_bytes, edited.history_bytes);
    assert_eq!(saved.documents_bytes, edited.documents_bytes);
}
