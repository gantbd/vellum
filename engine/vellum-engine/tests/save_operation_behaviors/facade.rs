use super::super::{catalog_support as catalog, discovery_support as discovery};
use super::*;
use vellum_engine::instrumentation::NativeTimingProfileV1;
fn catalog_writer(z: &mut Setup, annotation: bool) -> CatalogWriterBindingV1 {
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Write);
    cfg.identity_known = true;
    let address = catalog::address(ResourceCatalogTableV1::Write, "writer");
    z.f.owner
        .configure_resource_catalog_v1(
            &[ResourceCatalogRowV1 {
                address: catalog::address(ResourceCatalogTableV1::Write, "writer"),
                generation: 0,
                configuration: cfg,
            }],
            catalog::limits(),
            &catalog::facts(),
            &mut z.f.recorder,
        )
        .unwrap();
    z.f.owner
        .bind_catalog_writer_v1(
            z.f.port
                .source_binding_target(if annotation { b"comments" } else { b"source" })
                .unwrap(),
            &address,
            &TrustedCatalogWriterBindingV1 {
                profile: catalog::limits(),
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
            },
            &mut z.f.recorder,
        )
        .unwrap()
}
fn scope() -> DiscoveryScopeV1 {
    let mut context = context();
    context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context,
        policy_epoch: 7,
    }
}
fn visibility() -> NativeBuiltinEntryVisibilityV1<2> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
            order: Some(i),
            enabled: None,
        }),
    }
}
fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_ordinary_save_discovery_cut_v1(&scope(), &visibility(), &discovery::facts(), r)
        .unwrap()
}
struct Route {
    context: DiscoveryContextIdV1,
    entry: DiscoveryEntryIdV1,
    variants: [DiscoveryVariantIdV1; 2],
}
impl Route {
    fn install(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        o.configure_discovery_v1(&scope(), discovery::limits(), &discovery::facts(), r)
            .unwrap();
        let p = prepare(o, r);
        let context = o
            .acquire_discovery_v1(&scope(), &p, &discovery::facts(), r)
            .unwrap()
            .context;
        let p = prepare(o, r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                &discovery::facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        let entry = entries[0].reference.clone();
        let p = prepare(o, r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &entry },
                &discovery::facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(descriptor.variants.len(), 2);
        Self {
            context,
            entry,
            variants: std::array::from_fn(|i| descriptor.variants[i].reference.clone()),
        }
    }
    fn request(
        &self,
        o: &mut SourceCoordinator,
        variant: usize,
        command: NativeDiscoveredOrdinarySaveCommandV1<'_>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1, DiscoveredSaveOperationErrorV1> {
        let p = prepare(o, r);
        let result = o.request_discovered_ordinary_save_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &scope(),
                cut: &p,
                entry: &self.entry,
                variant: &self.variants[variant],
                facts: &discovery::facts(),
            },
            command,
            r,
        );
        // Wrong-family recognition fails before consuming the preparation.
        if matches!(result, Err(DiscoveredSaveOperationErrorV1::Discovery(_))) {
            o.retire_discovery_cut_v1(&p, &discovery::facts(), r)
                .unwrap();
        }
        result
    }
}
#[test]
fn installed_source_and_annotation_save_routes_recover_one_capture_and_release_actual_observed_results()
 {
    for annotation in [false, true] {
        for catalog_enabled in [false, true] {
            for capacity in [0, 1, 512] {
                let mut z = Setup::new();
                z.f.owner
                    .configure_retention_budget(33554432, &mut z.f.recorder)
                    .unwrap();
                z.f.owner
                    .configure_native_operation_observation_v1(
                        NativeOperationObservationPoolLimitsV1 {
                            max_contexts: 4,
                            max_issued: 4,
                            max_pool_bytes: 65536,
                        },
                        &mut z.f.recorder,
                    )
                    .unwrap();
                let writer = if catalog_enabled {
                    Some(catalog_writer(&mut z, annotation))
                } else {
                    None
                };
                let route = Route::install(&mut z.f.owner, &mut z.f.recorder);
                z.f.recorder = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                let req = request(&z.d, &z.source, b"PRIVATE-SOURCE");
                let ann = ann_request(
                    &z.d,
                    z.set,
                    &z.encoding,
                    &z.destination,
                    &z.source,
                    b"PRIVATE-ANNOTATIONS",
                );
                let common = current();
                let a = admission(&common);
                let ar = ann_release(&z.c);
                let sr = release(&z.c);
                let aa = ann_admission();
                let before = z.f.owner.resource_usage();
                let parent_before = z.f.owner.save_operation_usage_v1();
                let wrong = if annotation {
                    NativeDiscoveredOrdinarySaveCommandV1::Annotations {
                        request: &ann,
                        writer: writer.as_ref(),
                        release: &ar,
                        admission: None,
                        candidate: None,
                        pair: None,
                    }
                } else {
                    NativeDiscoveredOrdinarySaveCommandV1::Source {
                        request: &req,
                        writer: writer.as_ref(),
                        release: &sr,
                        admission: None,
                        pair: None,
                    }
                };
                assert_eq!(
                    route
                        .request(
                            &mut z.f.owner,
                            usize::from(!annotation),
                            wrong,
                            &mut z.f.recorder
                        )
                        .err(),
                    Some(DiscoveredSaveOperationErrorV1::Discovery(
                        DiscoveryErrorV1::Unavailable
                    ))
                );
                assert_eq!(z.f.owner.resource_usage(), before);
                assert_eq!(z.f.owner.save_operation_usage_v1(), parent_before);
                let pair = z.f.pair();
                let encoded = if annotation {
                    Some(candidate(&mut z.f, &z.e, &z.d, z.set))
                } else {
                    None
                };
                let command = if annotation {
                    NativeDiscoveredOrdinarySaveCommandV1::Annotations {
                        request: &ann,
                        writer: writer.as_ref(),
                        release: &ar,
                        admission: Some(&aa),
                        candidate: encoded,
                        pair,
                    }
                } else {
                    NativeDiscoveredOrdinarySaveCommandV1::Source {
                        request: &req,
                        writer: writer.as_ref(),
                        release: &sr,
                        admission: Some(&a),
                        pair: pair.as_ref(),
                    }
                };
                let first = route
                    .request(
                        &mut z.f.owner,
                        usize::from(annotation),
                        command,
                        &mut z.f.recorder,
                    )
                    .unwrap();
                assert_eq!(first.status, SaveOperationAdmissionStatusV1::Admitted);
                let after = z.f.owner.resource_usage();
                let policy = observation::policy(&z.c);
                let facts = observation::facts(&z.c);
                let retention = z.f.owner.retention_usage();
                let observer =
                    z.f.owner
                        .open_native_save_observation_v1(
                            &first.task,
                            &policy,
                            observation::obs_limits(),
                            &facts,
                            &mut z.f.recorder,
                        )
                        .unwrap();
                let mut denied = observation::facts(&z.c);
                denied.operation_receipt_allowed = None;
                assert!(
                    z.f.owner
                        .query_native_operation_observation_v1(
                            &observer,
                            NativeOperationObservationQueryV1::RetainedOutcome,
                            &policy,
                            &denied,
                            &mut z.f.recorder
                        )
                        .is_err()
                );
                let edit =
                    z.f.owner
                        .prepare(
                            &[DocumentEdits {
                                expected: z.d.clone(),
                                edits: vec![Replacement {
                                    start: 0,
                                    end: 0,
                                    expected: "".into(),
                                    inserted: "later ".into(),
                                }],
                            }],
                            &[],
                            &mut z.f.recorder,
                        )
                        .unwrap();
                z.f.owner.commit(edit, &mut z.f.recorder).unwrap();
                if catalog_enabled {
                    let plan = z
                        .f
                        .owner
                        .prepare_resource_catalog_v1(
                            &[ResourceCatalogChangeV1 {
                                address: catalog::address(ResourceCatalogTableV1::Write, "writer"),
                                expected: CatalogExpectationV1::Present { generation: 0 },
                                action: ResourceCatalogActionV1::Remove,
                            }],
                            &catalog::facts(),
                            &mut z.f.recorder,
                        )
                        .unwrap();
                    z.f.owner
                        .commit_resource_catalog_v1(plan, &catalog::facts(), &mut z.f.recorder)
                        .unwrap();
                }
                let command = if annotation {
                    NativeDiscoveredOrdinarySaveCommandV1::Annotations {
                        request: &ann,
                        writer: writer.as_ref(),
                        release: &ar,
                        admission: None,
                        candidate: None,
                        pair: None,
                    }
                } else {
                    NativeDiscoveredOrdinarySaveCommandV1::Source {
                        request: &req,
                        writer: writer.as_ref(),
                        release: &sr,
                        admission: None,
                        pair: None,
                    }
                };
                let replay = route
                    .request(
                        &mut z.f.owner,
                        usize::from(annotation),
                        command,
                        &mut z.f.recorder,
                    )
                    .unwrap();
                assert_eq!(
                    replay.status,
                    SaveOperationAdmissionStatusV1::ObserveExisting
                );
                assert_eq!(replay.task, first.task);
                assert_eq!(z.f.owner.resource_usage(), after);
                let mut changed_source = request(&z.d, &z.source, b"PRIVATE-SOURCE");
                changed_source.origin.attempt = b"different";
                let mut changed_annotations = ann_request(
                    &z.d,
                    z.set,
                    &z.encoding,
                    &z.destination,
                    &z.source,
                    b"PRIVATE-ANNOTATIONS",
                );
                changed_annotations.origin.attempt = b"different";
                let changed = if annotation {
                    NativeDiscoveredOrdinarySaveCommandV1::Annotations {
                        request: &changed_annotations,
                        writer: writer.as_ref(),
                        release: &ar,
                        admission: None,
                        candidate: None,
                        pair: None,
                    }
                } else {
                    NativeDiscoveredOrdinarySaveCommandV1::Source {
                        request: &changed_source,
                        writer: writer.as_ref(),
                        release: &sr,
                        admission: None,
                        pair: None,
                    }
                };
                assert_eq!(
                    route
                        .request(
                            &mut z.f.owner,
                            usize::from(annotation),
                            changed,
                            &mut z.f.recorder
                        )
                        .err(),
                    Some(DiscoveredSaveOperationErrorV1::Native(
                        ResourceError::Unavailable
                    ))
                );
                assert_eq!(z.f.owner.resource_usage(), after);
                let NativeOperationObservationReplyV1::Receipt {
                    receipt: NativeOperationReceiptV1::Save(receipt),
                    ..
                } =
                    z.f.owner
                        .query_native_operation_observation_v1(
                            &observer,
                            NativeOperationObservationQueryV1::RetainedOutcome,
                            &policy,
                            &facts,
                            &mut z.f.recorder,
                        )
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(
                    receipt.kind,
                    if annotation {
                        NativeSaveKindV1::AnnotationSave
                    } else {
                        NativeSaveKindV1::SourceSave
                    }
                );
                assert_eq!(receipt.phase, NativeSaveWritePhaseV1::Queued);
                assert_eq!(receipt.application, Application::Unknown);
                assert!(!receipt.requested_strength_satisfied);
                z.f.owner
                    .stop_native_operation_observation_v1(
                        &observer,
                        &observation::control(&z.c),
                        &mut z.f.recorder,
                    )
                    .unwrap();
                assert_eq!(
                    z.f.owner
                        .save_outcome(&first.task, &mut z.f.recorder)
                        .unwrap()
                        .phase,
                    SavePhase::Queued
                );
                z.f.owner
                    .retire_native_operation_observation_v1(
                        &observer,
                        &observation::control(&z.c),
                        &mut z.f.recorder,
                    )
                    .unwrap();
                assert_eq!(z.f.owner.retention_usage(), retention);
                let artifact =
                    z.f.owner
                        .save_outcome(&first.task, &mut z.f.recorder)
                        .unwrap()
                        .artifact;
                z.f.owner.stop_save(&first.task, &mut z.f.recorder).unwrap();
                z.f.owner
                    .retire_save(&first.task, &mut z.f.recorder)
                    .unwrap();
                if annotation {
                    z.f.owner
                        .retire_resource_artifact(&artifact, &mut z.f.recorder)
                        .unwrap();
                }
                assert_eq!(z.f.owner.save_operation_usage_v1().retired, 1);
                let events: Vec<_> = std::iter::from_fn(|| z.f.recorder.pop_front()).collect();
                assert!(events.len() <= capacity);
                assert!(!format!("{events:?}").contains("PRIVATE"));
                if capacity == 512 {
                    for b in [
                        Boundary::DiscoveryRequestRecognition,
                        Boundary::SaveOperationAdmission,
                        Boundary::SaveOperationRecovery,
                    ] {
                        assert!(events.iter().any(|e| e.boundary == b));
                    }
                }
            }
        }
    }
}
