use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
use crate::coordinator::shutdown::tests::{
    context, facts as shutdown_facts, limits as fence_limits,
};
use crate::instrumentation::NativeTimingProfileV1;

fn visibility() -> NativeBuiltinEntryVisibilityV1<1> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: [NativeBuiltinVariantVisibilityV1 {
            order: Some(0),
            enabled: None,
        }],
    }
}
fn prepare(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    native: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if native {
        return o
            .prepare_native_shutdown_disposition_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F18-shutdown-disposition",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |input| {
        o.prepare_discovery_cut_v1(s, input, &facts(), r)
    })
    .unwrap()
}
fn install(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    native: bool,
    r: &mut Recorder,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    DiscoveryVariantIdV1,
) {
    let mut l = limits();
    l.max_context_bytes = 256;
    l.max_row_bytes = 32768;
    l.max_capture_bytes = 32768;
    l.max_packet_bytes = 65536;
    l.max_reply_bytes = 65536;
    l.max_selectors = 32;
    l.max_issued = 128;
    l.metadata.max_bytes = 16384;
    l.metadata.max_nodes = 512;
    l.metadata.max_depth = 12;
    l.max_held_bytes = l.declared_bytes().unwrap();
    o.configure_discovery_v1(s, l, &facts(), r).unwrap();
    let cut = prepare(o, s, native, r);
    let id = o
        .acquire_discovery_v1(s, &cut, &facts(), r)
        .unwrap()
        .context;
    let cut = prepare(o, s, native, r);
    let DiscoveryReplyV1::Page { mut entries, .. } = o
        .query_discovery_v1(
            &id,
            s,
            &cut,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    let entry = entries.remove(0).reference;
    let cut = prepare(o, s, native, r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
        .query_discovery_v1(
            &id,
            s,
            &cut,
            DiscoveryQueryV1::Inspect { entry: &entry },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(descriptor.variants[0].schema.is_some());
    let variant = descriptor.variants.into_iter().next().unwrap().reference;
    (id, entry, variant)
}
fn inventory_limits() -> ShutdownInventoryLimitsV1 {
    ShutdownInventoryLimitsV1 {
        max_documents: 2,
        max_writes: 0,
        max_artifacts: 0,
        max_bindings: 0,
        max_reads: 0,
        max_owners: 64,
        max_attachments: 0,
        max_dependencies: 64,
        max_work: 4096,
        max_capture_bytes: 131072,
    }
}
fn coverage() -> TrustedDisposalCoverageV1 {
    TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: None,
        source_discard_allowed: Some(true),
        annotation_retention_ready: None,
        annotation_discard_allowed: Some(true),
        retention_complete: Some(true),
        effects_retained: Some(true),
        history_retained: Some(true),
        snapshots_retained: Some(true),
        providers_retained: Some(true),
        views_detachable: Some(true),
        memberships_detachable: Some(true),
        observer_scopes_fenced: Some(true),
        binding_coverage_complete: Some(true),
    }
}
fn request<T>(
    fence: &ShutdownFenceIdV1,
    document: &DocumentId,
    typed: bool,
    consume: impl FnOnce(&ShutdownDispositionEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use ShutdownDispositionEnvelopeValueV1 as V;
    let args = [
        (
            "contract",
            V::Data(D::Text(SHUTDOWN_DISPOSITION_PROFILE_V1)),
        ),
        ("operation", V::Data(D::Text("shutdown-disposition"))),
        ("fence", V::Fence(fence)),
        ("document", V::Document(document)),
    ];
    let header = [
        if typed {
            ("family", V::Data(D::Number(17.0)))
        } else {
            ("name", V::Data(D::Text("control-execution")))
        },
        ("variant", V::Data(D::Text("shutdown-disposition"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ];
    consume(&V::Object(&outer))
}

#[test]
fn installed_shutdown_disposition_uses_original_consent_current_rights_and_native_cleanup() {
    for native in [false, true] {
        for typed in [false, true] {
            for mode in 0..4 {
                let mut o = SourceCoordinator::new(CoordinatorLimits {
                    max_document_incarnations: 2,
                    max_participants: 2,
                    max_staged_bytes: 128,
                    source: crate::source::Limits {
                        max_source_bytes: 64,
                        max_edits: 4,
                        max_edit_payload_bytes: 64,
                    },
                });
                let mut setup = Recorder::disabled();
                let d = o.create(b"PRIVATE_PROSE", &mut setup).unwrap();
                o.configure_retention_budget(16777216, &mut setup).unwrap();
                let s = scope();
                let (id, entry, variant) = install(&mut o, &s, native, &mut setup);
                let c = context();
                let control = shutdown_facts(&c);
                let auth = TrustedShutdownInventoryFactsV1 {
                    control: &control,
                    inventory_readable: Some(true),
                };
                let basis = o
                    .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut setup)
                    .unwrap();
                let inventory = o
                    .capture_shutdown_inventory_v1(&basis, inventory_limits(), &auth, &mut setup)
                    .unwrap();
                let rows = o
                    .read_shutdown_inventory_v1(&inventory, &auth, &mut setup)
                    .unwrap();
                let input = [ShutdownDocumentDecisionV1 {
                    expected: &rows.documents[0],
                    source: Some(ShutdownDocumentChoiceV1::Discard),
                    annotations: Some(ShutdownDocumentChoiceV1::Discard),
                }];
                let ShutdownDecisionCaptureV1::Captured(decisions) = o
                    .capture_shutdown_decisions_v1(&inventory, &input, 65536, &auth, &mut setup)
                    .unwrap()
                else {
                    panic!()
                };
                let bindings = [ShutdownSaveBindingsV1 {
                    document: d.document.clone(),
                    source: None,
                    annotations: None,
                }];
                let ShutdownAcceptanceV1::Accepted(fence) = o
                    .accept_shutdown_disposition_v1(
                        &inventory, &decisions, &bindings, 262144, &auth, &mut setup,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                let sentinel = Observation {
                    boundary: Boundary::Control,
                    disposition: Disposition::Completed,
                    service_time: None,
                };
                if mode == 3 {
                    r.record(sentinel);
                }
                let mut published = None;
                for allowed in [false, true, true] {
                    let cut = prepare(&mut o, &s, native, &mut setup);
                    let before = (o.generation(), o.retention_usage());
                    let mut rights = coverage();
                    rights.source_discard_allowed = Some(allowed);
                    let result = request(&fence, &d.document, typed, |e| {
                        o.request_discovered_shutdown_disposition_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &id,
                                scope: &s,
                                cut: &cut,
                                entry: &entry,
                                variant: &variant,
                                facts: &facts(),
                            },
                            e,
                            &auth,
                            &rights,
                            &mut r,
                        )
                    });
                    if !native {
                        assert_eq!(
                            result,
                            Err(DiscoveredShutdownDispositionErrorV1::Discovery(
                                Error::Unavailable
                            ))
                        );
                        o.retire_discovery_cut_v1(&cut, &facts(), &mut setup)
                            .unwrap();
                        assert_eq!((o.generation(), o.retention_usage()), before);
                    } else if !allowed {
                        assert!(result.is_err());
                        assert_eq!((o.generation(), o.retention_usage()), before);
                    } else if published.is_some() {
                        assert!(
                            result.is_err(),
                            "original consent does not renew after disposal"
                        );
                        assert_eq!((o.generation(), o.retention_usage()), before);
                    } else {
                        let state = result.unwrap();
                        assert_eq!(state.phase, LifecyclePhase::DisposedRetained);
                        published = Some(state);
                    }
                }
                if mode == 0 {
                    assert_eq!((r.queued(), r.dropped()), (0, 0));
                } else if mode == 3 {
                    assert_eq!(r.pop_front(), Some(sentinel));
                    assert!(r.dropped() > 0);
                } else {
                    let mut dispatch = Vec::new();
                    while let Some(e) = r.pop_front() {
                        assert!(!format!("{e:?}").contains("PRIVATE_PROSE"));
                        if e.boundary == Boundary::DiscoveryShutdownDispositionDispatch {
                            assert_eq!(e.service_time.is_some(), mode == 2);
                            dispatch.push(e.disposition);
                        }
                    }
                    assert_eq!(
                        dispatch,
                        if native {
                            vec![
                                Disposition::Refused,
                                Disposition::Completed,
                                Disposition::Refused,
                            ]
                        } else {
                            vec![Disposition::Refused; 3]
                        }
                    );
                }
                // Closing discovery cannot strand protected original native progress.
                o.stop_discovery_context_v1(&id, &s, &facts(), &mut setup)
                    .unwrap();
                o.retire_discovery_context_v1(&id, &facts(), &mut setup)
                    .unwrap();
                o.stop_discovery_admission_v1(&facts(), &mut setup).unwrap();
                o.retire_discovery_registry_v1(&facts(), &mut setup)
                    .unwrap();
                if !native {
                    request(&fence, &d.document, typed, |e| {
                        o.request_decoded_shutdown_disposition_v1(e, &auth, &coverage(), &mut setup)
                            .unwrap()
                    });
                }
                let state = o.lifecycle_state(&d.document, &mut setup).unwrap();
                o.release_disposed_provenance(&d.document, state, &mut setup)
                    .unwrap();
                let state = o.lifecycle_state(&d.document, &mut setup).unwrap();
                o.reclaim_disposed_document_v1(
                    &d.document,
                    state,
                    &TrustedDocumentReclamationV1 {
                        authorized: Some(true),
                        authored_disposition_ready: Some(true),
                        session_references_released: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    },
                    &mut setup,
                )
                .unwrap();
                o.retire_shutdown_disposition_v1(
                    &fence,
                    &control,
                    Some(true),
                    Some(true),
                    &mut setup,
                )
                .unwrap();
                assert_eq!(
                    o.finish_shutdown_attempt_v1(&fence, inventory_limits(), &auth, &mut setup),
                    Ok(ShutdownAttemptStatusV1::Drained)
                );
                o.retire_shutdown_attempt_report_v1(
                    &fence,
                    &control,
                    Some(true),
                    Some(true),
                    &mut setup,
                )
                .unwrap();
                o.retire_shutdown_control_v1(&fence, inventory_limits(), &auth, &mut setup)
                    .unwrap();
                assert_eq!(o.retention_usage().reserved, 0);
                assert!(!o.is_faulted());
            }
        }
    }
}
