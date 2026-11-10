use super::serialization_tests::fixture;
use super::*;
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}
#[test]
fn source_capacity_refusal_is_before_any_task_or_artifact_issuance() {
    let (mut o, mut r, d, _) = fixture();
    o.resources
        .limits
        .as_mut()
        .unwrap()
        .max_future_baseline_bytes = 0;
    let before = o.resource_usage();
    assert_eq!(
        o.queue_pinned_save(&d, requirements(), &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(o.resource_usage(), before);
    assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
}
#[test]
fn source_capacity_reserves_a_real_generation_before_issuance() {
    let (mut o, mut r, d, _) = fixture();
    o.generation = u64::MAX;
    let before = o.resource_usage();
    assert_eq!(
        o.queue_pinned_save(&d, requirements(), &mut r),
        Err(ResourceError::VersionExhausted)
    );
    assert_eq!(o.resource_usage(), before);
}
#[test]
fn source_capacity_is_real_pending_storage_and_queued_stop_releases_it_once() {
    let (mut o, mut r, d, _) = fixture();
    let id = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    assert_eq!(o.resource_usage().pending_baseline_publications, 1);
    assert!(o.resource_usage().reserved_baseline_bytes > 0);
    assert_eq!(
        o.save_outcome(&id, &mut r).unwrap().baseline_capacity,
        Some(BaselineCapacityState::Pending)
    );
    o.stop_save(&id, &mut r).unwrap();
    o.stop_save(&id, &mut r).unwrap();
    assert_eq!(o.resource_usage().pending_baseline_publications, 0);
    assert_eq!(o.resource_usage().reserved_baseline_bytes, 0);
    assert_eq!(
        o.save_outcome(&id, &mut r).unwrap().baseline_capacity,
        Some(BaselineCapacityState::NotWritten)
    );
}
#[test]
fn source_capacity_installs_at_reserved_root_and_last_generation_after_admission_fence() {
    for capacity in [0, 1, 128] {
        let (mut o, mut setup, d, port) = fixture();
        o.generation = u64::MAX - 1;
        let id = o.queue_pinned_save(&d, requirements(), &mut setup).unwrap();
        let required = o.semantic_root_usage(&mut setup).unwrap().total_bytes
            + o.pending_baseline_root_growth(BaselineRootProjection::default())
                .unwrap();
        assert_eq!(
            o.configure_semantic_root_budget(required - 1, &mut setup),
            Err(RootAccountingErrorV1::Limit)
        );
        o.configure_semantic_root_budget(required, &mut setup)
            .unwrap();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        let before_registry = o.resource_usage();
        assert!(matches!(
            o.register_provider(
                ProviderProfile {
                    synchronization_supported: false
                },
                None,
                &mut r
            ),
            Err(ResourceError::Limit)
        ));
        assert_eq!(o.resource_usage(), before_registry);
        o.stop_source_save_admission_v1(&mut r);
        assert!(o.source_save_admission_stopped_v1());
        let before = o.resource_usage();
        assert_eq!(
            o.queue_pinned_save(&d, requirements(), &mut r),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(o.resource_usage(), before);
        let dispatch = o
            .dispatch_save(&id, port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
            .unwrap();
        for evidence in [
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(&[b'v'; 64]),
            )
            .unwrap(),
            port.no_future_effects(&dispatch).unwrap(),
            port.quiescence(&dispatch).unwrap(),
        ] {
            o.record_save_evidence(evidence, &mut r).unwrap();
        }
        drop(dispatch);
        o.publish_save_baseline(&id, &mut r).unwrap();
        assert_eq!(o.generation(), u64::MAX);
        assert_eq!(o.resource_usage().pending_baseline_publications, 0);
        assert_eq!(o.resource_usage().reserved_baseline_bytes, 0);
        assert_eq!(
            o.save_outcome(&id, &mut r).unwrap().baseline_capacity,
            Some(BaselineCapacityState::Installed)
        );
        o.publish_save_baseline(&id, &mut r).unwrap();
        assert_eq!(o.generation(), u64::MAX);
        assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, required);
        assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Retained));
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 128 {
            assert!(
                std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary == Boundary::SaveBaseline
                    && e.disposition == Disposition::Completed
                    && e.service_time.is_some())
            );
        }
    }
}
#[test]
fn source_capacity_unknown_effect_retains_promise_and_terminal_proof_releases_without_healing() {
    let (mut o, mut r, d, port) = fixture();
    let id = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = o
        .dispatch_save(&id, port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    o.stop_save(&id, &mut r).unwrap();
    assert_eq!(
        o.release_source_save_capacity_v1(&id, &mut r),
        Err(ResourceError::Retained)
    );
    assert_eq!(o.resource_usage().pending_baseline_publications, 1);
    assert_eq!(
        o.release_annotation_save_capacity(&id, &mut r),
        Err(ResourceError::Unsupported)
    );
    for evidence in [
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
        o.record_save_evidence(evidence, &mut r).unwrap();
    }
    drop(dispatch);
    o.faulted = true;
    assert_eq!(
        o.release_source_save_capacity_v1(&id, &mut r).unwrap(),
        BaselineCapacityState::NotWritten
    );
    assert_eq!(
        o.release_source_save_capacity_v1(&id, &mut r).unwrap(),
        BaselineCapacityState::NotWritten
    );
    assert!(o.faulted);
    assert_eq!(o.resource_usage().reserved_baseline_bytes, 0);
    o.retire_save(&id, &mut r).unwrap();
    assert!(o.faulted);
}
#[test]
fn source_capacity_supersession_requires_an_actual_intact_newer_installed_baseline() {
    for corrupt in [false, true] {
        let (mut o, mut r, d, port) = fixture();
        let older = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let newer = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        for id in [&older, &newer] {
            let dispatch = o
                .dispatch_save(id, port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
                .unwrap();
            for evidence in [
                port.result(
                    &dispatch,
                    Application::Written,
                    1,
                    Synchronization::NotRequested,
                    Some(b"v1"),
                )
                .unwrap(),
                port.no_future_effects(&dispatch).unwrap(),
                port.quiescence(&dispatch).unwrap(),
            ] {
                o.record_save_evidence(evidence, &mut r).unwrap();
            }
        }
        assert_eq!(
            o.release_source_save_capacity_v1(&older, &mut r),
            Err(ResourceError::Retained)
        );
        assert_eq!(
            o.publish_save_baseline(&older, &mut r),
            Err(ResourceError::Superseded)
        );
        o.publish_save_baseline(&newer, &mut r).unwrap();
        if corrupt {
            o.resources.bindings[0].baseline.as_mut().unwrap().bytes =
                Blob(Arc::from(&b"wrong"[..]));
            let before = o.resource_usage().reserved_baseline_bytes;
            assert_eq!(
                o.release_source_save_capacity_v1(&older, &mut r),
                Err(ResourceError::Faulted)
            );
            assert_eq!(o.resource_usage().reserved_baseline_bytes, before);
            assert!(o.faulted);
        } else {
            assert_eq!(
                o.release_source_save_capacity_v1(&older, &mut r).unwrap(),
                BaselineCapacityState::Superseded
            );
            assert_eq!(o.resource_usage().reserved_baseline_bytes, 0);
            o.retire_save(&older, &mut r).unwrap();
            assert_eq!(o.retire_save(&newer, &mut r), Err(ResourceError::Retained));
        }
    }
}
