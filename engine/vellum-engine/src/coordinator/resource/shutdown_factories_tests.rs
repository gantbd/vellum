use super::*;
use crate::coordinator::shutdown::tests::install;
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}
fn save_limits() -> SaveOperationLimitsV1 {
    SaveOperationLimitsV1 {
        max_issued: 8,
        max_label_bytes: 64,
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_store_bytes: 131072,
    }
}
fn continuity() -> SourceContinuityLimitsV1 {
    SourceContinuityLimitsV1 {
        max_edges: 8,
        max_derivation_bytes: 65536,
        max_receipt_bytes: 65536,
        max_retained_bytes: 262144,
    }
}

#[test]
fn shutdown_blocks_every_provider_registration_route_without_revoking_original_save_delivery() {
    let (mut o, mut r, d, p) = serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let task = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    install(&mut o, &mut r);
    let before = o.resource_usage();
    let profile = ProviderProfile {
        synchronization_supported: false,
    };
    assert!(matches!(
        o.register_provider(profile, None, &mut r),
        Err(ResourceError::Unavailable)
    ));
    assert!(matches!(
        o.register_provider_with_destination_capabilities(
            profile,
            ProviderDestinationCapabilitiesV1::default(),
            None,
            &mut r
        ),
        Err(ResourceError::Unavailable)
    ));
    assert!(matches!(
        o.register_provider_with_causal_capabilities(
            profile,
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1::default(),
            None,
            &mut r
        ),
        Err(ResourceError::Unavailable)
    ));
    assert_eq!(o.resource_usage(), before);
    let dispatch = o
        .dispatch_save(&task, p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    for e in [
        p.result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        p.no_future_effects(&dispatch).unwrap(),
        p.quiescence(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, &mut r).unwrap();
    }
    drop(dispatch);
    o.publish_save_baseline(&task, &mut r).unwrap();
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_blocks_codec_registration_and_new_annotation_artifacts_but_preserves_original_bytes() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, _, read, mut r) = read::tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let source = read::tests::original_source(&o, &read);
        let encoder = o.register_annotation_encoder(&mut r).unwrap();
        let snapshot = o.capture_annotations(&source, 0, &mut r).unwrap();
        let artifact = o
            .retain_annotation_artifact(
                encoder.seal(snapshot.clone(), b"abc".to_vec()).unwrap(),
                &mut r,
            )
            .unwrap();
        install(&mut o, &mut r);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = o.resource_usage();
        assert!(matches!(
            o.register_annotation_encoder(&mut r),
            Err(ResourceError::Unavailable)
        ));
        assert!(matches!(
            o.register_annotation_restorer(&mut r),
            Err(ResourceError::Unavailable)
        ));
        assert!(matches!(
            o.register_annotation_restorer_for_encoding(&encoder.context(), &mut r),
            Err(ResourceError::Unavailable)
        ));
        assert!(matches!(
            o.retain_annotation_artifact(encoder.seal(snapshot, b"new".to_vec()).unwrap(), &mut r),
            Err(ResourceError::Unavailable)
        ));
        assert_eq!(o.resource_usage(), before);
        assert_eq!(
            o.resource_artifact_bytes(&artifact, 0, 3, &mut r).unwrap(),
            b"abc"
        );
        o.stop_resource_artifact(&artifact, &mut r).unwrap();
        o.retire_resource_artifact(&artifact, &mut r).unwrap();
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::ArtifactAdmission
                        && e.disposition == Disposition::Refused)
            );
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::ArtifactDelivery
                        && e.disposition == Disposition::Completed)
            );
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn shutdown_refuses_annotation_export_admission_but_original_export_can_settle_and_retire() {
    let (mut o, p, read, mut r) = read::tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let source = read::tests::original_source(&o, &read);
    let encoder = o.register_annotation_encoder(&mut r).unwrap();
    let snapshot = o.capture_annotations(&source, 0, &mut r).unwrap();
    let artifact = o
        .retain_annotation_artifact(encoder.seal(snapshot, b"abc".to_vec()).unwrap(), &mut r)
        .unwrap();
    let task = o
        .queue_annotation_export(
            &artifact,
            p.read_result(b"copy", b"v0", b"old").unwrap(),
            requirements(),
            &mut r,
        )
        .unwrap();
    install(&mut o, &mut r);
    let before = o.resource_usage();
    assert_eq!(
        o.queue_annotation_export(
            &artifact,
            p.read_result(b"other", b"v0", b"old").unwrap(),
            requirements(),
            &mut r
        ),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(o.resource_usage(), before);
    let dispatch = o
        .dispatch_save(
            &task,
            p.read_result(b"copy", b"v0", b"old").unwrap(),
            &mut r,
        )
        .unwrap();
    for e in [
        p.result(
            &dispatch,
            Application::Written,
            3,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        p.no_future_effects(&dispatch).unwrap(),
        p.quiescence(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, &mut r).unwrap();
    }
    drop(dispatch);
    o.retire_save(&task, &mut r).unwrap();
    o.retire_resource_artifact(&artifact, &mut r).unwrap();
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_blocks_fresh_save_and_continuity_profiles_but_matching_profiles_are_service() {
    for configured in [false, true] {
        let (mut o, mut r, _, _) = serialization_tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        if configured {
            o.configure_save_operations_v1(save_limits(), &mut r)
                .unwrap();
            o.configure_source_continuity_v1(continuity(), &mut r)
                .unwrap();
        }
        install(&mut o, &mut r);
        let before = (
            o.resource_usage(),
            o.save_operation_usage_v1(),
            o.retention_usage(),
        );
        let expected = if configured {
            Ok(())
        } else {
            Err(ResourceError::Unavailable)
        };
        assert_eq!(
            o.configure_save_operations_v1(save_limits(), &mut r),
            expected
        );
        assert_eq!(
            o.configure_source_continuity_v1(continuity(), &mut r),
            expected
        );
        assert_eq!(
            (
                o.resource_usage(),
                o.save_operation_usage_v1(),
                o.retention_usage()
            ),
            before
        );
    }
}

#[test]
fn shutdown_blocks_source_artifact_capture_and_materialization_without_discarding_existing_preparation()
 {
    for prepared in [false, true] {
        let (mut o, mut r, source, _) = serialization_tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let format = o.source_serialization(&source.document, &mut r).unwrap();
        let plan = if prepared {
            Some(o.prepare_source_artifact(&source, format, &mut r).unwrap())
        } else {
            None
        };
        install(&mut o, &mut r);
        let before = o.resource_usage();
        assert!(matches!(
            o.prepare_source_artifact(&source, format, &mut r),
            Err(ResourceError::Unavailable)
        ));
        if let Some(plan) = plan {
            assert_eq!(
                o.retain_prepared_source_artifact(&plan, &mut r),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(o.resource_usage(), before);
            o.discard_source_artifact_preparation(&plan, &mut r)
                .unwrap();
            assert_eq!(o.resource_usage().prepared_sources, 0);
        } else {
            assert_eq!(o.resource_usage(), before);
        }
        assert!(!o.is_faulted());
    }
}

#[test]
fn shutdown_refuses_new_and_prepared_annotation_restoration_including_empty_restoration() {
    for count in [0, 1] {
        for prepared in [false, true] {
            let (mut o, _, package, mut r) = loaded_baseline::commit_tests::ready(count);
            o.configure_retention_budget(1048576, &mut r).unwrap();
            let mut package = Some(package);
            let plan = prepared.then(|| {
                o.prepare_annotation_restoration_native(package.take().unwrap(), None, &mut r)
                    .unwrap()
            });
            install(&mut o, &mut r);
            let before = (
                o.resource_usage(),
                o.annotation_usage().retained_records,
                o.annotation_usage().present_records,
                o.generation(),
            );
            let result = if let Some(plan) = plan {
                o.commit_annotation_restoration_native(plan, &mut r)
            } else {
                o.restore_annotations(package.unwrap(), None, &mut r)
            };
            assert_eq!(result, Err(ResourceError::Unavailable));
            // Refusal consumes the caller-owned package; its independent source
            // snapshot may drop. Authored state and read provenance must not change.
            assert_eq!(
                (
                    o.resource_usage(),
                    o.annotation_usage().retained_records,
                    o.annotation_usage().present_records,
                    o.generation()
                ),
                before
            );
            assert!(!o.is_faulted());
        }
    }
}

#[test]
fn shutdown_preserves_qualified_baseline_adoption_from_an_already_restored_read() {
    let (mut o, read, package, mut r) = loaded_baseline::commit_tests::ready(0);
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.restore_annotations(package, None, &mut r).unwrap();
    o.attach_annotation_read_baselines(&mut r).unwrap();
    install(&mut o, &mut r);
    let before = o.generation();
    assert_eq!(
        o.adopt_loaded_annotation_baseline(&read, None, 16384, &mut r),
        Ok(true)
    );
    assert_eq!(o.generation(), before + 1);
    assert_eq!(
        o.adopt_loaded_annotation_baseline(&read, None, 16384, &mut r),
        Ok(false)
    );
    assert_eq!(o.generation(), before + 1);
    assert_eq!(o.create(b"new", &mut r), Err(CoordinatorError::Unavailable));
    assert!(!o.is_faulted());
}
