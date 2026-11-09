use super::super::{catalog_support as catalog, discovery_support as discovery};
use super::*;
use vellum_engine::instrumentation::NativeTimingProfileV1;
fn scope() -> DiscoveryScopeV1 {
    let mut context = context();
    context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context,
        policy_epoch: 7,
    }
}
fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_read_admission_discovery_cut_v1(
        &scope(),
        &NativeBuiltinEntryVisibilityV1 {
            order: Some(0),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
        &discovery::facts(),
        r,
    )
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
        command: NativeDiscoveredReadCommandV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiscoveredReadOperationAdmissionV1, DiscoveredReadOperationErrorV1> {
        let p = prepare(o, r);
        let result = o.request_discovered_read_v1(
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
        if matches!(result, Err(DiscoveredReadOperationErrorV1::Discovery(_))) {
            o.retire_discovery_cut_v1(&p, &discovery::facts(), r)
                .unwrap();
        }
        result
    }
}
fn source_facts(c: &SettingsExecutionContextV1) -> TrustedSourceReadOperationFactsV1<'_> {
    TrustedSourceReadOperationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        provider_available: Some(true),
        launch_allowed: Some(true),
    }
}
fn setup() -> Fixture {
    let mut f = Fixture::new();
    f.owner
        .configure_retention_budget(33554432, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_annotation_read_operations_v1(read_limits(), &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_reader_v1(
            SourceReaderLimitsV1 {
                max_live: 4,
                max_issued: 8,
                max_input_bytes: 128,
                max_chunks: 8,
                max_held_bytes: 65536,
                max_store_bytes: 131072,
            },
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .configure_source_read_operations_v1(
            SourceReadOperationLimitsV1 {
                max_capture_bytes: 8192,
                max_reply_bytes: 1024,
                max_label_bytes: 64,
                max_store_bytes: 1048576,
            },
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .configure_native_operation_observation_v1(
            NativeOperationObservationPoolLimitsV1 {
                max_contexts: 4,
                max_issued: 4,
                max_pool_bytes: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    f
}
fn targets(
    f: &mut Fixture,
    annotation: bool,
    bound: bool,
) -> (SourceReadTargetV1, AnnotationReadTarget) {
    let source = f.port.source_read_target_v1(b"PRIVATE-SOURCE").unwrap();
    let annotations = f.port.annotation_read_target(b"comments").unwrap();
    if !bound {
        return (source, annotations);
    }
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Read);
    cfg.identity_known = true;
    if !annotation {
        cfg.capabilities = ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
            atomic_read: false,
            insertion_line_break: CatalogInsertionLineBreakV1::Lf,
        };
    }
    let address = catalog::address(ResourceCatalogTableV1::Read, "reader");
    f.owner
        .configure_resource_catalog_v1(
            &[ResourceCatalogRowV1 {
                address: catalog::address(ResourceCatalogTableV1::Read, "reader"),
                generation: 0,
                configuration: cfg,
            }],
            catalog::limits(),
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    if annotation {
        let target = f
            .owner
            .bind_catalog_annotation_read_target_v1(
                annotations,
                &address,
                &TrustedCatalogAnnotationReadBindingV1 {
                    profile: catalog::limits(),
                    expected_generation: 0,
                    max_binding_bytes: 16384,
                    authorized: Some(true),
                    scope: Some(true),
                    continuous: Some(true),
                    live: Some(true),
                    identity_allowed: Some(true),
                    read_allowed: Some(true),
                    annotation_readable: Some(true),
                    provider_available: Some(true),
                    correspondence_current: Some(true),
                    resources_ready: Some(true),
                },
                &mut f.recorder,
            )
            .unwrap();
        (source, target)
    } else {
        let target = f
            .owner
            .bind_catalog_source_read_target_v1(
                source,
                &address,
                &TrustedCatalogSourceReadBindingV1 {
                    profile: catalog::limits(),
                    expected_generation: 0,
                    max_binding_bytes: 16384,
                    authorized: Some(true),
                    scope: Some(true),
                    continuous: Some(true),
                    live: Some(true),
                    identity_allowed: Some(true),
                    read_allowed: Some(true),
                    provider_available: Some(true),
                    correspondence_current: Some(true),
                    resources_ready: Some(true),
                },
                &mut f.recorder,
            )
            .unwrap();
        (target, annotations)
    }
}
#[test]
fn installed_reads_recover_original_tasks_and_observe_completed_evidence_without_activation() {
    for annotation in [false, true] {
        for bound in [false, true] {
            for capacity in [0, 1, 512] {
                let mut f = setup();
                let c = context();
                let (source, annotations) = targets(&mut f, annotation, bound);
                let route = Route::install(&mut f.owner, &mut f.recorder);
                f.recorder = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                let d = f.doc.clone();
                let annotation_binding = f.port.source_binding_target(b"comments").unwrap();
                let source_binding = f.port.source_binding_target(b"source").unwrap();
                let sq = OwnedSourceReadRequestV1 {
                    origin: SourceReadOperationOriginV1 {
                        operation: b"PRIVATE-READ",
                        effect: b"read",
                        attempt: b"one",
                    },
                    target: &source,
                    strength: ReadStrength::CapturedRead,
                };
                let aq = read_request(
                    &d,
                    &annotations,
                    &annotation_binding,
                    &source_binding,
                    b"PRIVATE-READ",
                );
                let sf = source_facts(&c);
                let af = facts(&c);
                let command = || {
                    if annotation {
                        NativeDiscoveredReadCommandV1::Annotations {
                            request: &aq,
                            facts: &af,
                            pair: None,
                        }
                    } else {
                        NativeDiscoveredReadCommandV1::Source {
                            request: &sq,
                            facts: &sf,
                        }
                    }
                };
                let before = f.owner.resource_usage();
                let sr_before = f.owner.source_reader_usage_v1().unwrap();
                assert_eq!(
                    route
                        .request(
                            &mut f.owner,
                            usize::from(!annotation),
                            command(),
                            &mut f.recorder
                        )
                        .err(),
                    Some(DiscoveredReadOperationErrorV1::Discovery(
                        DiscoveryErrorV1::Unavailable
                    ))
                );
                assert_eq!(f.owner.resource_usage(), before);
                assert_eq!(f.owner.source_reader_usage_v1().unwrap(), sr_before);
                let pair = f.pair();
                let first = route
                    .request(
                        &mut f.owner,
                        usize::from(annotation),
                        if annotation {
                            NativeDiscoveredReadCommandV1::Annotations {
                                request: &aq,
                                facts: &af,
                                pair,
                            }
                        } else {
                            command()
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
                let generation = f.owner.generation();
                let retention = f.owner.retention_usage();
                let p = observation::policy(&c);
                let of = observation::facts(&c);
                let observer = match &first {
                    DiscoveredReadOperationAdmissionV1::Source(v) => {
                        assert_eq!(v.status, SourceReadOperationAdmissionStatusV1::Admitted);
                        let dispatch = f
                            .owner
                            .submit_source_read_v1(&v.task, &mut f.recorder)
                            .unwrap();
                        let version = SourceReadVersionV1::Opaque(b"v");
                        for e in [
                            f.port.source_read_start_v1(&dispatch, version).unwrap(),
                            f.port
                                .source_read_chunk_v1(&dispatch, 0, 0, b"PRIVATE-BODY")
                                .unwrap(),
                            f.port
                                .source_read_end_v1(&dispatch, version, 1, 12)
                                .unwrap(),
                            f.port.source_read_quiescence_v1(&dispatch).unwrap(),
                        ] {
                            f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
                        }
                        drop(dispatch);
                        f.owner
                            .open_native_source_read_observation_v1(
                                &v.task,
                                &p,
                                observation::obs_limits(),
                                &of,
                                &mut f.recorder,
                            )
                            .unwrap()
                    }
                    DiscoveredReadOperationAdmissionV1::Annotations(v) => {
                        assert_eq!(v.status, AnnotationReadOperationAdmissionStatusV1::Admitted);
                        let pair = f.pair();
                        let dispatch = f
                            .owner
                            .submit_annotation_read(&v.task, pair, &mut f.recorder)
                            .unwrap();
                        for e in [
                            f.port.annotation_read_start(&dispatch, b"v").unwrap(),
                            f.port
                                .annotation_read_chunk(&dispatch, 0, 0, b"PRIVATE-BODY")
                                .unwrap(),
                            f.port.annotation_read_end(&dispatch, b"v", 1, 12).unwrap(),
                            f.port.annotation_read_quiescence(&dispatch).unwrap(),
                        ] {
                            f.owner.record_annotation_read(e, &mut f.recorder).unwrap();
                        }
                        drop(dispatch);
                        f.owner
                            .open_native_annotation_read_observation_v1(
                                &v.task,
                                &p,
                                observation::obs_limits(),
                                &of,
                                &mut f.recorder,
                            )
                            .unwrap()
                    }
                };
                assert_eq!(f.owner.generation(), generation);
                let mut denied = observation::facts(&c);
                denied.operation_receipt_allowed = None;
                assert!(
                    f.owner
                        .query_native_operation_observation_v1(
                            &observer,
                            NativeOperationObservationQueryV1::RetainedOutcome,
                            &p,
                            &denied,
                            &mut f.recorder
                        )
                        .is_err()
                );
                let edit = f
                    .owner
                    .prepare(
                        &[DocumentEdits {
                            expected: d.clone(),
                            edits: vec![Replacement {
                                start: 0,
                                end: 0,
                                expected: "".into(),
                                inserted: "later ".into(),
                            }],
                        }],
                        &[],
                        &mut f.recorder,
                    )
                    .unwrap();
                f.owner.commit(edit, &mut f.recorder).unwrap();
                if bound {
                    let plan = f
                        .owner
                        .prepare_resource_catalog_v1(
                            &[ResourceCatalogChangeV1 {
                                address: catalog::address(ResourceCatalogTableV1::Read, "reader"),
                                expected: CatalogExpectationV1::Present { generation: 0 },
                                action: ResourceCatalogActionV1::Remove,
                            }],
                            &catalog::facts(),
                            &mut f.recorder,
                        )
                        .unwrap();
                    f.owner
                        .commit_resource_catalog_v1(plan, &catalog::facts(), &mut f.recorder)
                        .unwrap();
                }
                let usage = f.owner.resource_usage();
                let sr_usage = f.owner.source_reader_usage_v1().unwrap();
                let replay = route
                    .request(
                        &mut f.owner,
                        usize::from(annotation),
                        command(),
                        &mut f.recorder,
                    )
                    .unwrap();
                match (&first, &replay) {
                    (
                        DiscoveredReadOperationAdmissionV1::Source(a),
                        DiscoveredReadOperationAdmissionV1::Source(b),
                    ) => {
                        assert_eq!(a.task, b.task);
                        assert_eq!(
                            b.status,
                            SourceReadOperationAdmissionStatusV1::ObserveExisting
                        );
                    }
                    (
                        DiscoveredReadOperationAdmissionV1::Annotations(a),
                        DiscoveredReadOperationAdmissionV1::Annotations(b),
                    ) => {
                        assert_eq!(a.task, b.task);
                        assert_eq!(
                            b.status,
                            AnnotationReadOperationAdmissionStatusV1::ObserveExisting
                        );
                    }
                    _ => panic!(),
                }
                assert_eq!(f.owner.resource_usage(), usage);
                assert_eq!(f.owner.source_reader_usage_v1().unwrap(), sr_usage);
                let changed_source = OwnedSourceReadRequestV1 {
                    origin: SourceReadOperationOriginV1 {
                        operation: b"PRIVATE-READ",
                        effect: b"read",
                        attempt: b"different",
                    },
                    target: &source,
                    strength: ReadStrength::CapturedRead,
                };
                let mut changed_annotations = read_request(
                    &d,
                    &annotations,
                    &annotation_binding,
                    &source_binding,
                    b"PRIVATE-READ",
                );
                changed_annotations.origin.attempt = b"different";
                let changed = if annotation {
                    NativeDiscoveredReadCommandV1::Annotations {
                        request: &changed_annotations,
                        facts: &af,
                        pair: None,
                    }
                } else {
                    NativeDiscoveredReadCommandV1::Source {
                        request: &changed_source,
                        facts: &sf,
                    }
                };
                assert_eq!(
                    route
                        .request(
                            &mut f.owner,
                            usize::from(annotation),
                            changed,
                            &mut f.recorder
                        )
                        .err(),
                    Some(DiscoveredReadOperationErrorV1::Native(
                        ResourceError::Unavailable
                    ))
                );
                assert_eq!(f.owner.resource_usage(), usage);
                assert_eq!(f.owner.source_reader_usage_v1().unwrap(), sr_usage);
                let NativeOperationObservationReplyV1::Receipt { receipt, .. } = f
                    .owner
                    .query_native_operation_observation_v1(
                        &observer,
                        NativeOperationObservationQueryV1::RetainedOutcome,
                        &p,
                        &of,
                        &mut f.recorder,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                match receipt {
                    NativeOperationReceiptV1::SourceLoad(v) if !annotation => {
                        assert_eq!(v.received_bytes, 12);
                        assert!(v.quiescent);
                        assert!(!v.stopped);
                    }
                    NativeOperationReceiptV1::AnnotationLoad(v) if annotation => {
                        assert_eq!(v.received_bytes, 12);
                        assert!(v.quiescent);
                        assert!(!v.stopped);
                    }
                    _ => panic!(),
                }
                f.owner
                    .stop_native_operation_observation_v1(
                        &observer,
                        &observation::control(&c),
                        &mut f.recorder,
                    )
                    .unwrap();
                f.owner
                    .retire_native_operation_observation_v1(
                        &observer,
                        &observation::control(&c),
                        &mut f.recorder,
                    )
                    .unwrap();
                assert_eq!(f.owner.retention_usage(), retention);
                match &first {
                    DiscoveredReadOperationAdmissionV1::Source(v) => {
                        f.owner
                            .stop_source_read_v1(&v.task, &mut f.recorder)
                            .unwrap();
                        f.owner
                            .retire_source_read_v1(&v.task, &mut f.recorder)
                            .unwrap();
                    }
                    DiscoveredReadOperationAdmissionV1::Annotations(v) => {
                        f.owner
                            .stop_annotation_read(&v.task, &mut f.recorder)
                            .unwrap();
                        f.owner
                            .retire_annotation_read(&v.task, &mut f.recorder)
                            .unwrap();
                    }
                }
                assert_eq!(
                    route
                        .request(
                            &mut f.owner,
                            usize::from(annotation),
                            command(),
                            &mut f.recorder
                        )
                        .err(),
                    Some(DiscoveredReadOperationErrorV1::Native(
                        ResourceError::Unavailable
                    ))
                );
                let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
                assert!(events.len() <= capacity);
                assert!(!format!("{events:?}").contains("PRIVATE"));
                if capacity == 512 {
                    assert!(events.iter().any(|v| v.boundary
                        == if annotation {
                            Boundary::AnnotationReadOperationRecovery
                        } else {
                            Boundary::SourceReadOperationRecovery
                        }));
                }
            }
        }
    }
}
