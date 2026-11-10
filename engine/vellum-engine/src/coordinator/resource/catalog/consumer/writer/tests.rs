use super::*;
use crate::coordinator::resource::catalog::tests::{address, config, facts, limits};
#[test]
fn catalog_annotation_export_clone_is_exactly_bounded_and_releases_only_its_own_hold() {
    for delta in [-1, 0, 1] {
        let (mut o, p, old, mut r) = crate::coordinator::resource::read::tests::fixture();
        let d = crate::coordinator::resource::read::tests::original_source(&o, &old);
        o.stop_annotation_read(&old, &mut r).unwrap();
        o.retire_annotation_read(&old, &mut r).unwrap();
        let a = address(ResourceCatalogTableV1::Write, "writer");
        let mut cfg = config(ResourceCatalogTableV1::Write, "key");
        cfg.identity_known = true;
        o.configure_resource_catalog_v1(
            &[ResourceCatalogRowV1 {
                address: a.copy().unwrap(),
                generation: 0,
                configuration: cfg,
            }],
            limits(),
            &facts(),
            &mut r,
        )
        .unwrap();
        let writer = o
            .bind_catalog_writer_v1(
                p.source_binding_target(b"comments").unwrap(),
                &a,
                &binding(),
                &mut r,
            )
            .unwrap();
        let encoder = o.register_annotation_encoder(&mut r).unwrap();
        let snapshot = o.capture_annotations(&d, 0, &mut r).unwrap();
        let original = o
            .retain_annotation_artifact(encoder.seal(snapshot, b"[]".to_vec()).unwrap(), &mut r)
            .unwrap();
        let exact = writer.capture.native_bytes().unwrap() + 4;
        o.resources.limits.as_mut().unwrap().max_held_artifact_bytes =
            exact.checked_add_signed(delta).unwrap();
        let before = o.resource_usage();
        let result = o.queue_catalog_annotation_export_v1(
            &original,
            p.read_result(b"comments", b"v0", b"{}").unwrap(),
            requirements(),
            &writer,
            &mut r,
        );
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(o.resource_usage(), before);
        } else {
            let id = result.unwrap();
            assert_eq!(o.resource_usage().held_artifact_bytes, exact);
            o.stop_save(&id, &mut r).unwrap();
            o.retire_save(&id, &mut r).unwrap();
            assert_eq!(o.resource_usage().held_artifact_bytes, 2);
        }
        assert_eq!(
            o.resource_artifact_bytes(&original, 0, 2, &mut r).unwrap(),
            b"[]"
        );
    }
}
fn binding() -> TrustedCatalogWriterBindingV1 {
    TrustedCatalogWriterBindingV1 {
        profile: limits(),
        expected_generation: 0,
        max_binding_bytes: 16384,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        identity_allowed: Some(true),
        write_allowed: Some(true),
        provider_available: Some(true),
        correspondence_current: Some(true),
        resources_ready: Some(true),
    }
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}
fn setup(
    known: bool,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    ProviderPort,
    ResourceCatalogAddressV1,
) {
    let (mut o, mut r, d, p) = crate::coordinator::resource::serialization_tests::fixture();
    let a = address(ResourceCatalogTableV1::Write, "writer");
    let mut cfg = config(ResourceCatalogTableV1::Write, "catalog-key");
    cfg.identity_known = known;
    o.configure_resource_catalog_v1(
        &[ResourceCatalogRowV1 {
            address: a.copy().unwrap(),
            generation: 0,
            configuration: cfg,
        }],
        limits(),
        &facts(),
        &mut r,
    )
    .unwrap();
    (o, r, d, p, a)
}
#[test]
fn exact_binding_and_artifact_hold_limits_precede_all_writer_issuance() {
    let (mut o, mut r, d, p, a) = setup(true);
    let b = o
        .bind_catalog_writer_v1(
            p.source_binding_target(b"key").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    let capture = b.capture.native_bytes().unwrap();
    let mut f = binding();
    f.max_binding_bytes = capture - 1;
    assert!(matches!(
        o.bind_catalog_writer_v1(p.source_binding_target(b"key").unwrap(), &a, &f, &mut r),
        Err(ResourceError::Limit)
    ));
    f.max_binding_bytes = capture;
    o.bind_catalog_writer_v1(p.source_binding_target(b"key").unwrap(), &a, &f, &mut r)
        .unwrap();
    let total = capture + 1; // Original source payload is exactly A.
    o.resources.limits.as_mut().unwrap().max_held_artifact_bytes = total - 1;
    let before = o.resource_usage();
    assert_eq!(
        o.queue_catalog_pinned_source_save_v1(&d, &b, requirements(), None, &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(o.resource_usage(), before);
    o.resources.limits.as_mut().unwrap().max_held_artifact_bytes = total;
    let id = o
        .queue_catalog_pinned_source_save_v1(&d, &b, requirements(), None, &mut r)
        .unwrap();
    assert_eq!(o.resource_usage().held_artifact_bytes, total);
    let artifact = o.save_outcome(&id, &mut r).unwrap().artifact;
    assert_eq!(
        o.retire_resource_artifact(&artifact, &mut r),
        Err(ResourceError::Retained)
    );
    o.stop_save(&id, &mut r).unwrap();
    o.retire_save(&id, &mut r).unwrap();
    assert_eq!(
        o.retire_resource_artifact(&artifact, &mut r),
        Err(ResourceError::Unavailable)
    ); // Source retirement already released its artifact.
    assert_eq!(o.resource_usage().held_artifact_bytes, 0);
    assert_eq!(o.resource_usage().issued_lifetime, 1);
}
#[test]
fn unknown_identity_cannot_admit_a_write_and_full_capture_integrity_is_independent_from_row_currentness()
 {
    let (mut o, mut r, d, p, a) = setup(false);
    let b = o
        .bind_catalog_writer_v1(
            p.source_binding_target(b"key").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.queue_catalog_pinned_source_save_v1(&d, &b, requirements(), None, &mut r),
        Err(ResourceError::Unavailable)
    );
    let plan = o
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: a,
                expected: CatalogExpectationV1::Present { generation: 0 },
                action: ResourceCatalogActionV1::Remove,
            }],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(plan, &facts(), &mut r)
        .unwrap();
    assert!(
        b.capture
            .intact(&o.namespace, &p.registration, &b.capture.identity)
            .is_ok()
    );
    assert_eq!(
        o.writer_catalog_current(
            Some(&b.capture),
            &p.registration,
            &b.capture.identity,
            false,
            false,
            &mut r
        ),
        Err(ResourceError::Stale)
    );
    let other = p.source_binding_target(b"different").unwrap();
    assert_eq!(
        b.capture
            .intact(&o.namespace, &other.registration, &other.identity),
        Err(ResourceError::Faulted)
    );
}

#[test]
fn owned_parent_capture_ceiling_counts_the_full_catalog_before_child_or_origin_issuance() {
    for delta in [-1, 0, 1] {
        let (mut o, mut r, d, p, a) = setup(true);
        let writer = o
            .bind_catalog_writer_v1(
                p.source_binding_target(b"key").unwrap(),
                &a,
                &binding(),
                &mut r,
            )
            .unwrap();
        let c = crate::coordinator::serialization::tests::context();
        let base =
            crate::coordinator::resource::operations::tests::source_capsule_bytes_for_test(&c);
        let exact = base + writer.capture.native_bytes().unwrap();
        o.configure_save_operations_v1(
            SaveOperationLimitsV1 {
                max_issued: 4,
                max_label_bytes: 64,
                max_capture_bytes: exact.checked_add_signed(delta).unwrap(),
                max_reply_bytes: 1024,
                max_store_bytes: 131072,
            },
            &mut r,
        )
        .unwrap();
        let target = p.source_binding_target(b"key").unwrap();
        let native = OwnedSourceSaveRequestV1 {
            origin: SaveOperationOriginV1 {
                operation: b"op",
                effect: b"e",
                attempt: b"a",
            },
            source: &d,
            expected_serialization: 1,
            expected_binding: 0,
            target: &target,
            requirements: requirements(),
            condition: SaveOperationConditionV1::Pinned,
        };
        let request = OwnedCatalogSourceSaveRequestV1 {
            native: &native,
            writer: &writer,
        };
        let release = TrustedSaveOperationReleaseV1 {
            context: &c,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            write_allowed: Some(true),
            source_readable: Some(true),
            disposition_readable: Some(true),
            retained: Some(true),
            reply_ready: Some(true),
            transfer_ready: Some(true),
            resources_ready: Some(true),
        };
        let current = TrustedSourceExportFactsV1 {
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            source_readable: Some(true),
            write_allowed: Some(true),
            binding_coverage_complete: Some(true),
            writer_coverage_complete: Some(true),
            provider_available: Some(true),
            resources_ready: Some(true),
            launch_allowed: Some(true),
            destination_allowed: None,
            identity_allowed: Some(true),
            companions_ready: Some(true),
            versions_available: Some(true),
            reply_ready: Some(true),
            create_allowed: None,
            overwrite_allowed: None,
        };
        let admission = TrustedOrdinarySourceSaveAdmissionV1 {
            current: &current,
            owner_control_allowed: Some(true),
            preparation_reserved: Some(true),
            ordinary_bindings_current: Some(true),
            ordinary_identity_current: Some(true),
        };
        let before = o.resource_usage();
        let result = o.request_owned_catalog_source_save_v1(
            &request,
            &release,
            Some(&admission),
            None,
            &mut r,
        );
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(o.resource_usage(), before);
            assert_eq!(o.save_operation_usage_v1().issued, 0);
        } else {
            result.unwrap();
            assert_eq!(o.save_operation_usage_v1().issued, 1);
        }
    }
}

fn export_facts() -> TrustedSourceExportFactsV1 {
    TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: Some(true),
        overwrite_allowed: Some(true),
    }
}
#[test]
fn catalog_source_transfer_checks_additional_hold_before_issuing_and_discards_only_prepaid_source_on_refusal()
 {
    for delta in [-1, 0, 1] {
        let (mut o, mut r, d, p, a) = setup(true);
        let writer = o
            .bind_catalog_writer_v1(
                p.source_binding_target(b"key").unwrap(),
                &a,
                &binding(),
                &mut r,
            )
            .unwrap();
        let exact = writer.capture.native_bytes().unwrap() + 1;
        o.resources.limits.as_mut().unwrap().max_held_artifact_bytes =
            exact.checked_add_signed(delta).unwrap();
        let format = o.source_serialization(&d.document, &mut r).unwrap();
        let prepared = o.prepare_source_artifact(&d, format, &mut r).unwrap();
        let generation = o.generation();
        let result = o.queue_catalog_source_export_v1(
            &prepared,
            p.read_result(b"key", b"v0", b"A")
                .unwrap()
                .into_source_export_destination(),
            SourceExportOptionsV1 {
                purpose: SourceExportPurposeV1::CopyExport,
                requirements: requirements(),
                pair: None,
            },
            &writer,
            &export_facts(),
            &mut r,
        );
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(o.resource_usage().issued_lifetime, 0);
            assert_eq!(o.resource_usage().issued_artifacts_lifetime, 0);
            assert_eq!(o.resource_usage().held_artifact_bytes, 0);
            assert_eq!(o.generation(), generation);
        } else {
            result.unwrap();
            assert_eq!(o.resource_usage().held_artifact_bytes, exact);
            assert_eq!(o.resource_usage().source_export_reservations, 1);
        }
        assert_eq!(o.resource_usage().prepared_sources, 0);
    }
}
