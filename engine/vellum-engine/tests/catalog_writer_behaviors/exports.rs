use super::*;
#[test]
fn export_create_conditional_and_typed_after_declarations_are_independent_of_native_capabilities() {
    for mode in 0..3 {
        for declared in [false, true] {
            let mut cfg = catalog::configuration(ResourceCatalogTableV1::Write);
            cfg.identity_known = true;
            if let ResourceCatalogCapabilitiesV1::Write {
                exclusive_create,
                conditional_write,
                causal_receipts,
                ..
            } = &mut cfg.capabilities
            {
                *exclusive_create = true;
                *conditional_write = true;
                *causal_receipts = true;
                match mode {
                    0 => *exclusive_create = declared,
                    1 => *conditional_write = declared,
                    _ => *causal_receipts = declared,
                }
            }
            let mut f = setup_configuration(cfg);
            f.port = f
                .owner
                .register_provider_with_causal_capabilities(
                    ProviderProfile {
                        synchronization_supported: false,
                    },
                    ProviderDestinationCapabilitiesV1 {
                        exclusive_create: true,
                        atomic_conditional_write: true,
                    },
                    ProviderCausalCapabilitiesV1 {
                        causal_receipts: true,
                        non_reuse_tokens: false,
                    },
                    None,
                    &mut f.recorder,
                )
                .unwrap();
            let writer = binding(&mut f, b"export");
            let p = prepare(&mut f);
            let mut options = options(SourceExportPurposeV1::CopyExport);
            if mode == 1 {
                options.requirements.protection = Protection::AtomicConditional;
            }
            let result = if mode == 1 {
                f.owner.queue_catalog_source_export_v1(
                    &p,
                    destination(&f),
                    options,
                    &writer,
                    &facts(),
                    &mut f.recorder,
                )
            } else {
                f.owner.queue_catalog_source_export_v2(
                    &p,
                    f.port.absent_source_export_destination(b"export").unwrap(),
                    SourceExportOptionsV2 {
                        intent: options,
                        result_version_profile: if mode == 2 {
                            SaveResultVersionProfileV1::EpochToken
                        } else {
                            SaveResultVersionProfileV1::Opaque
                        },
                    },
                    &writer,
                    &facts(),
                    &mut f.recorder,
                )
            };
            if declared {
                let id = result.unwrap();
                assert_eq!(
                    f.owner
                        .save_outcome(&id, &mut f.recorder)
                        .unwrap()
                        .result_version_profile,
                    if mode == 2 {
                        SaveResultVersionProfileV1::EpochToken
                    } else {
                        SaveResultVersionProfileV1::Opaque
                    }
                );
            } else {
                assert_eq!(result, Err(ResourceError::Unsupported));
                assert_eq!(f.owner.resource_usage().issued_lifetime, 0);
                assert_eq!(f.owner.resource_usage().source_export_reservations, 0);
                assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
            }
        }
    }
}
fn facts() -> TrustedSourceExportFactsV1 {
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
fn prepare(f: &mut support::Fixture) -> PreparedSourceArtifactIdV1 {
    let s = f
        .owner
        .source_serialization(&f.doc.document, &mut f.recorder)
        .unwrap();
    f.owner
        .prepare_source_artifact(&f.doc, s, &mut f.recorder)
        .unwrap()
}
fn options(purpose: SourceExportPurposeV1) -> SourceExportOptionsV1 {
    SourceExportOptionsV1 {
        purpose,
        requirements: requirements(),
        pair: None,
    }
}
fn destination(f: &support::Fixture) -> SourceExportDestinationV1 {
    f.port
        .read_result(b"export", b"old", b"OLD")
        .unwrap()
        .into_source_export_destination()
}
#[test]
fn stale_catalog_export_consumes_only_its_preparation_without_task_or_reservation_issuance() {
    let mut f = setup();
    let binding = binding(&mut f, b"export");
    remove(&mut f);
    let p = prepare(&mut f);
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner.queue_catalog_source_export_v1(
            &p,
            destination(&f),
            options(SourceExportPurposeV1::CopyExport),
            &binding,
            &facts(),
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    let after = f.owner.resource_usage();
    assert_eq!(after.issued_lifetime, before.issued_lifetime);
    assert_eq!(
        after.source_export_reservations,
        before.source_export_reservations
    );
    assert_eq!(after.prepared_sources, 0);
    assert_eq!(after.held_artifact_bytes, 0);
}

fn control() -> TrustedSourceExportControlV1 {
    TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    }
}
fn export_pair(f: &mut support::Fixture) -> Option<OrdinaryPairEvidence> {
    let destination = destination(f);
    f.owner
        .source_export_pair_check(&f.doc.document, &destination, &mut f.recorder)
        .unwrap()
        .map(|p| {
            f.port
                .attest_ordinary_pair(&p, ResourceRelation::Distinct)
                .unwrap()
        })
}
#[test]
fn dispatched_catalog_copy_keeps_original_receipts_and_can_release_and_retire_after_withdrawal() {
    let mut f = setup();
    let binding = binding(&mut f, b"export");
    let p = prepare(&mut f);
    let id = f
        .owner
        .queue_catalog_source_export_v1(
            &p,
            destination(&f),
            options(SourceExportPurposeV1::CopyExport),
            &binding,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f
        .owner
        .dispatch_source_export(&id, destination(&f), None, &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(dispatch.bytes(), b"text");
    remove(&mut f);
    finish(&mut f, &dispatch, 4);
    assert!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .quiescent
    );
    f.owner
        .release_source_export_reservation(&id, &control(), &mut f.recorder)
        .unwrap();
    drop(dispatch);
    f.owner.retire_save(&id, &mut f.recorder).unwrap();
    let usage = f.owner.resource_usage();
    assert_eq!(usage.held_artifact_bytes, 0);
    assert_eq!(usage.source_export_reservations, 0);
    assert_eq!(usage.issued_lifetime, 1);
}
#[test]
fn save_as_checks_catalog_again_at_activation_and_preserves_already_activated_history() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
    for (withdraw_first, capacity) in [false, true]
        .into_iter()
        .flat_map(|v| [0, 1, 128].map(|c| (v, c)))
    {
        let mut f = setup();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let binding = binding(&mut f, b"export");
        let p = prepare(&mut f);
        let mut options = options(SourceExportPurposeV1::SaveAs);
        options.pair = export_pair(&mut f);
        let id = f
            .owner
            .queue_catalog_source_export_v1(
                &p,
                destination(&f),
                options,
                &binding,
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        let pair = export_pair(&mut f);
        let dispatch = f
            .owner
            .dispatch_source_export(&id, destination(&f), pair, &facts(), &mut f.recorder)
            .unwrap();
        if withdraw_first {
            remove(&mut f);
        }
        finish(&mut f, &dispatch, 4);
        let generation = f.owner.generation();
        let pair = export_pair(&mut f);
        let facts = TrustedSourceSaveAsFactsV1 {
            control: control(),
            probe_attested: Some(true),
            resource_continuity_attested: Some(true),
            create_allowed: Some(true),
            overwrite_allowed: Some(true),
        };
        let outcome = f.owner.activate_source_save_as(
            &id,
            f.port
                .source_save_as_probe(b"export", Some(b"next"))
                .unwrap(),
            pair,
            8,
            &facts,
            &mut f.recorder,
        );
        if withdraw_first {
            assert_eq!(outcome, Err(ResourceError::Stale));
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(
                f.owner
                    .save_outcome(&id, &mut f.recorder)
                    .unwrap()
                    .source_save_as_binding_applied,
                Some(false)
            );
        } else {
            assert_eq!(outcome, Ok(true));
            let saved = f
                .owner
                .saved_state(&f.doc.document, &mut f.recorder)
                .unwrap();
            remove(&mut f);
            assert_eq!(
                f.owner
                    .saved_state(&f.doc.document, &mut f.recorder)
                    .unwrap(),
                saved
            );
            assert_eq!(
                f.owner
                    .save_outcome(&id, &mut f.recorder)
                    .unwrap()
                    .source_save_as_binding_applied,
                Some(true)
            );
        }
        let mut checks = 0;
        while let Some(o) = f.recorder.pop_front() {
            if o.boundary == Boundary::CatalogWriterCurrentness {
                checks += 1;
                assert!(o.service_time.is_some());
            }
        }
        if capacity == 128 {
            // Admission and dispatch each check once. Save As preparation
            // checks currentness, then a successful fenced commit checks again;
            // withdrawal refuses during preparation before that final check.
            assert_eq!(checks, if withdraw_first { 3 } else { 4 });
        }
        if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn catalog_bound_annotation_export_publishes_only_under_current_original_registration() {
    let mut f = setup();
    let writer = binding(&mut f, b"comments");
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let original = f
        .owner
        .retain_annotation_artifact(
            encoder.seal(snapshot, b"[]".to_vec()).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair();
    let id = f
        .owner
        .queue_catalog_bound_annotation_export_v1(
            &original,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            requirements(),
            &writer,
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &id,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    finish(&mut f, &dispatch, 2);
    let pair = f.pair();
    assert_eq!(
        f.owner
            .publish_annotation_baseline(&id, pair, 16384, &mut f.recorder),
        Ok(true)
    );
    remove(&mut f);
    let pair = f.pair();
    assert_eq!(
        f.owner
            .publish_annotation_baseline(&id, pair, 16384, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    assert!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .quiescent
    );
}
#[test]
fn annotation_catalog_exports_keep_reusable_inputs_unchanged_and_own_separately_bounded_captures() {
    let mut f = setup();
    let writer = binding(&mut f, b"export");
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let encoded = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
    let original = f
        .owner
        .retain_annotation_artifact(encoded, &mut f.recorder)
        .unwrap();
    let id = f
        .owner
        .queue_catalog_annotation_export_v1(
            &original,
            f.port.read_result(b"export", b"old", b"OLD").unwrap(),
            requirements(),
            &writer,
            &mut f.recorder,
        )
        .unwrap();
    let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
    assert_ne!(outcome.artifact, original);
    assert_eq!(
        f.owner
            .resource_artifact_bytes(&original, 0, 2, &mut f.recorder)
            .unwrap(),
        b"[]"
    );
    remove(&mut f);
    let before = f.owner.resource_usage();
    assert!(matches!(
        f.owner.dispatch_save(
            &id,
            f.port.read_result(b"export", b"old", b"OLD").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    ));
    assert_eq!(f.owner.resource_usage(), before);
    f.owner.stop_save(&id, &mut f.recorder).unwrap();
    f.owner.retire_save(&id, &mut f.recorder).unwrap();
    assert_eq!(f.owner.resource_usage().held_artifact_bytes, 2);
    f.owner
        .retire_resource_artifact(&original, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
}
