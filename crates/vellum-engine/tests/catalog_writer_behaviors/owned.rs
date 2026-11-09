use super::*;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "save".into(),
        destination: "d".into(),
        profile: "native/1".into(),
        continuity: "c".into(),
    }
}
fn configure(f: &mut support::Fixture) {
    f.owner
        .configure_save_operations_v1(
            SaveOperationLimitsV1 {
                max_issued: 8,
                max_label_bytes: 64,
                max_capture_bytes: 8192,
                max_reply_bytes: 1024,
                max_store_bytes: 131072,
            },
            &mut f.recorder,
        )
        .unwrap();
}
fn release(c: &SettingsExecutionContextV1) -> TrustedSaveOperationReleaseV1<'_> {
    TrustedSaveOperationReleaseV1 {
        context: c,
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
    }
}
fn current() -> TrustedSourceExportFactsV1 {
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
        destination_allowed: None,
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: None,
    }
}
fn admission(c: &TrustedSourceExportFactsV1) -> TrustedOrdinarySourceSaveAdmissionV1<'_> {
    TrustedOrdinarySourceSaveAdmissionV1 {
        current: c,
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
    }
}
fn request<'a>(
    d: &'a DocumentVersion,
    target: &'a SourceBindingTargetV1,
) -> OwnedSourceSaveRequestV1<'a> {
    OwnedSourceSaveRequestV1 {
        origin: SaveOperationOriginV1 {
            operation: b"save",
            effect: b"write",
            attempt: b"one",
        },
        source: d,
        expected_serialization: 1,
        expected_binding: 0,
        target,
        requirements: requirements(),
        condition: SaveOperationConditionV1::Pinned,
    }
}
#[test]
fn owned_catalog_source_recovery_keeps_original_dependency_after_withdrawal_and_cannot_drop_it() {
    let mut f = setup();
    configure(&mut f);
    let binding = binding(&mut f, b"source");
    let target = f.port.source_binding_target(b"source").unwrap();
    let d = f.doc.clone();
    let native = request(&d, &target);
    let req = OwnedCatalogSourceSaveRequestV1 {
        native: &native,
        writer: &binding,
    };
    let c = context();
    let current = current();
    let admission = admission(&current);
    let pair = f.pair();
    let first = f
        .owner
        .request_owned_catalog_source_save_v1(
            &req,
            &release(&c),
            Some(&admission),
            pair.as_ref(),
            &mut f.recorder,
        )
        .unwrap();
    let usage = f.owner.resource_usage();
    let operation_usage = f.owner.save_operation_usage_v1();
    let equal = super::binding(&mut f, b"source");
    let equal_request = OwnedCatalogSourceSaveRequestV1 {
        native: &native,
        writer: &equal,
    };
    assert_eq!(
        f.owner
            .request_owned_catalog_source_save_v1(
                &equal_request,
                &release(&c),
                None,
                None,
                &mut f.recorder
            )
            .unwrap()
            .task,
        first.task
    );
    let wrong = super::binding(&mut f, b"different");
    let changed_request = OwnedCatalogSourceSaveRequestV1 {
        native: &native,
        writer: &wrong,
    };
    assert_eq!(
        f.owner.request_owned_catalog_source_save_v1(
            &changed_request,
            &release(&c),
            None,
            None,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    remove(&mut f);
    let again = f
        .owner
        .request_owned_catalog_source_save_v1(&req, &release(&c), None, None, &mut f.recorder)
        .unwrap();
    assert_eq!(again.task, first.task);
    assert_eq!(
        again.status,
        SaveOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(f.owner.resource_usage(), usage);
    assert_eq!(f.owner.save_operation_usage_v1(), operation_usage);
    assert_eq!(
        f.owner
            .request_owned_source_save_v1(&native, &release(&c), None, None, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    let pair = f.pair().unwrap();
    assert!(matches!(
        f.owner.dispatch_save_with_pair(
            &first.task,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    ));
}

fn annotation_release(
    c: &SettingsExecutionContextV1,
) -> TrustedAnnotationSaveOperationReleaseV1<'_> {
    TrustedAnnotationSaveOperationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        write_allowed: Some(true),
        annotation_readable: Some(true),
        disposition_readable: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
    }
}
fn annotation_admission() -> TrustedOrdinaryAnnotationSaveAdmissionV1 {
    TrustedOrdinaryAnnotationSaveAdmissionV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        complete_disclosure: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        destination_context_attested: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        aliases_current: Some(true),
        source_evidence_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        preparation_reserved: Some(true),
        owner_control_allowed: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
        historical_artifact_readable: Some(true),
        overwrite_destination_allowed: Some(true),
    }
}
#[test]
fn owned_catalog_annotation_recovery_keeps_original_capture_and_claim_without_a_new_candidate() {
    let mut f = setup();
    configure(&mut f);
    let binding = binding(&mut f, b"comments");
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let encoding = encoder.context();
    let d = f.doc.clone();
    let c = context();
    let snapshot = f.owner.capture_annotations(&d, 0, &mut f.recorder).unwrap();
    let candidate = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
    let destination = f.port.read_result(b"comments", b"a0", b"{}").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let native = OwnedAnnotationSaveRequestV1 {
        origin: SaveOperationOriginV1 {
            operation: b"annotation",
            effect: b"write",
            attempt: b"one",
        },
        source: &d,
        expected_set: 0,
        expected_annotation_binding: 1,
        expected_source_binding: Some(ExpectedSaveBindingV1 {
            target: &source,
            generation: 0,
        }),
        encoding: &encoding,
        destination: &destination,
        requirements: requirements(),
        condition: SaveOperationConditionV1::Pinned,
    };
    let request = OwnedCatalogAnnotationSaveRequestV1 {
        native: &native,
        writer: &binding,
    };
    let pair = f.pair();
    let first = f
        .owner
        .request_owned_catalog_annotation_save_v1(
            &request,
            &annotation_release(&c),
            Some(&annotation_admission()),
            Some(candidate),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let held = f.owner.resource_usage();
    remove(&mut f);
    let again = f
        .owner
        .request_owned_catalog_annotation_save_v1(
            &request,
            &annotation_release(&c),
            None,
            None,
            None,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        again.status,
        SaveOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(again.task, first.task);
    assert_eq!(f.owner.resource_usage(), held);
    assert_eq!(
        f.owner.request_owned_annotation_save_v1(
            &native,
            &annotation_release(&c),
            None,
            None,
            None,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    let pair = f.pair().unwrap();
    assert!(matches!(
        f.owner.dispatch_save_with_pair(
            &first.task,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    ));
}

fn causal_setup(cfg: ResourceCatalogConfigurationV1) -> support::Fixture {
    let mut f = setup_configuration(cfg);
    let p = f
        .owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: true,
            },
            None,
            &mut f.recorder,
        )
        .unwrap();
    f.doc = f
        .owner
        .load_resource(
            p.read_result_causal(
                b"key",
                ResourceVersionV1 {
                    epoch: b"epoch",
                    token: b"v0",
                },
                b"A",
            )
            .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.port = p;
    f.owner
        .configure_source_continuity_v1(
            SourceContinuityLimitsV1 {
                max_edges: 8,
                max_derivation_bytes: 8192,
                max_receipt_bytes: 16384,
                max_retained_bytes: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    configure(&mut f);
    f
}
fn chain_configuration() -> ResourceCatalogConfigurationV1 {
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Write);
    cfg.identity_known = true;
    if let ResourceCatalogCapabilitiesV1::Write {
        logical_continuity,
        causal_receipts,
        non_reuse_tokens,
        ..
    } = &mut cfg.capabilities
    {
        *logical_continuity = true;
        *causal_receipts = true;
        *non_reuse_tokens = true;
    }
    cfg
}
#[test]
fn catalog_own_chain_requires_each_declaration_and_actual_native_support_and_retains_original_intent()
 {
    for missing in 0..4 {
        let mut cfg = chain_configuration();
        if let ResourceCatalogCapabilitiesV1::Write {
            logical_continuity,
            causal_receipts,
            non_reuse_tokens,
            ..
        } = &mut cfg.capabilities
        {
            match missing {
                0 => *logical_continuity = false,
                1 => *causal_receipts = false,
                2 => *non_reuse_tokens = false,
                _ => {}
            }
        }
        let mut f = causal_setup(cfg);
        let binding = binding(&mut f, b"key");
        let target = f.port.source_binding_target(b"key").unwrap();
        let d = f.doc.clone();
        let c = context();
        let current = current();
        let mut native = request(&d, &target);
        native.condition = SaveOperationConditionV1::OwnChain;
        let request = OwnedCatalogSourceSaveRequestV1 {
            native: &native,
            writer: &binding,
        };
        let before = f.owner.resource_usage();
        let predecessor = if missing == 3 {
            Some(
                f.owner
                    .queue_catalog_pinned_source_save_v1(
                        &d,
                        &binding,
                        requirements(),
                        None,
                        &mut f.recorder,
                    )
                    .unwrap(),
            )
        } else {
            None
        };
        let result = f.owner.request_owned_catalog_source_save_v1(
            &request,
            &release(&c),
            Some(&admission(&current)),
            None,
            &mut f.recorder,
        );
        if missing != 3 {
            assert_eq!(result, Err(ResourceError::Unsupported));
            assert_eq!(f.owner.resource_usage(), before);
            assert_eq!(f.owner.save_operation_usage_v1().issued, 0);
        } else {
            let id = result.unwrap().task;
            let version = |token| ResourceVersionV1 {
                epoch: b"epoch",
                token,
            };
            let first = f
                .owner
                .dispatch_save(
                    &predecessor.unwrap(),
                    f.port
                        .read_result_causal(b"key", version(b"v0"), b"A")
                        .unwrap(),
                    &mut f.recorder,
                )
                .unwrap();
            for evidence in [
                f.port
                    .causal_edge(&first, version(b"v0"), version(b"v1"))
                    .unwrap(),
                f.port
                    .result_causal(
                        &first,
                        Application::Written,
                        1,
                        Synchronization::NotRequested,
                        Some(version(b"v1")),
                    )
                    .unwrap(),
                f.port.no_future_effects(&first).unwrap(),
            ] {
                f.owner
                    .record_save_evidence(evidence, &mut f.recorder)
                    .unwrap();
            }
            let dispatch = f
                .owner
                .dispatch_own_chain_save_v1(
                    &id,
                    f.port
                        .read_result_causal(
                            b"key",
                            ResourceVersionV1 {
                                epoch: b"epoch",
                                token: b"v1",
                            },
                            b"A",
                        )
                        .unwrap(),
                    None,
                    &TrustedSourceContinuityLaunchV1 {
                        causal_progression_allowed: Some(true),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(dispatch.bytes(), b"A");
            assert_eq!(dispatch.continuity_receipt().unwrap().edges().len(), 1);
            remove(&mut f);
            let recovered = f
                .owner
                .request_owned_catalog_source_save_v1(
                    &request,
                    &release(&c),
                    None,
                    None,
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(recovered.task, id);
            assert_eq!(
                recovered.status,
                SaveOperationAdmissionStatusV1::ObserveExisting
            );
        }
    }
    let mut f = setup_configuration(chain_configuration());
    let binding = binding(&mut f, b"source");
    let pair = f.pair();
    assert_eq!(
        f.owner.queue_catalog_own_chain_source_save_v1(
            &f.doc,
            &binding,
            requirements(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Unsupported)
    );
}

#[test]
fn optional_diagnostics_do_not_change_catalog_owned_execution_recovery_or_stale_fresh_admission() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
    for capacity in [0, 1, 128] {
        let mut f = setup();
        configure(&mut f);
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let binding = binding(&mut f, b"source");
        let target = f.port.source_binding_target(b"source").unwrap();
        let d = f.doc.clone();
        let native = request(&d, &target);
        let request = OwnedCatalogSourceSaveRequestV1 {
            native: &native,
            writer: &binding,
        };
        let c = context();
        let current = current();
        let admission = admission(&current);
        let pair = f.pair();
        let first = f
            .owner
            .request_owned_catalog_source_save_v1(
                &request,
                &release(&c),
                Some(&admission),
                pair.as_ref(),
                &mut f.recorder,
            )
            .unwrap();
        let dispatch = dispatch_source(&mut f, &first.task);
        remove(&mut f);
        finish(&mut f, &dispatch, 4);
        let usage = f.owner.resource_usage();
        let again = f
            .owner
            .request_owned_catalog_source_save_v1(
                &request,
                &release(&c),
                None,
                None,
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(again.task, first.task);
        assert_eq!(
            again.status,
            SaveOperationAdmissionStatusV1::ObserveExisting
        );
        let new_native = OwnedSourceSaveRequestV1 {
            origin: SaveOperationOriginV1 {
                operation: b"second",
                ..native.origin
            },
            ..native
        };
        let fresh = OwnedCatalogSourceSaveRequestV1 {
            native: &new_native,
            writer: &binding,
        };
        let pair = f.pair();
        assert_eq!(
            f.owner.request_owned_catalog_source_save_v1(
                &fresh,
                &release(&c),
                Some(&admission),
                pair.as_ref(),
                &mut f.recorder
            ),
            Err(ResourceError::Stale)
        );
        assert_eq!(f.owner.resource_usage(), usage);
        assert_eq!(f.owner.save_operation_usage_v1().issued, 1);
        let mut admissions = 0;
        let mut recoveries = 0;
        let mut currentness = 0;
        while let Some(o) = f.recorder.pop_front() {
            match o.boundary {
                Boundary::SaveOperationAdmission => admissions += 1,
                Boundary::SaveOperationRecovery => recoveries += 1,
                Boundary::CatalogWriterCurrentness => {
                    currentness += 1;
                    assert!(o.service_time.is_some());
                }
                _ => {}
            }
        }
        if capacity == 128 {
            assert_eq!(admissions, 2);
            assert_eq!(recoveries, 1);
            assert_eq!(currentness, 3);
        }
        if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
    }
}
