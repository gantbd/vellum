use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::observation_control::request_v3 as envelope;
use crate::coordinator::publication_stream::tests::{
    facts as stream_facts, limits as stream_limits, native, owner,
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
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_observation_revocation_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F19-context-revocation",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |i| {
        o.prepare_discovery_cut_v1(s, i, &facts(), r)
    })
    .unwrap()
}
fn install(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    DiscoveryVariantIdV1,
) {
    let mut l = limits();
    l.max_context_bytes = 256;
    l.max_entries = 1;
    l.max_variants = 1;
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
    let cut = prepare(o, s, installed, r);
    let context = o
        .acquire_discovery_v1(s, &cut, &facts(), r)
        .unwrap()
        .context;
    let cut = prepare(o, s, installed, r);
    let DiscoveryReplyV1::Page { mut entries, .. } = o
        .query_discovery_v1(
            &context,
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
    let cut = prepare(o, s, installed, r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
        .query_discovery_v1(
            &context,
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
    assert_eq!(descriptor.variants.len(), 1);
    assert!(descriptor.variants[0].schema.is_some());
    let variant = descriptor.variants.into_iter().next().unwrap().reference;
    (context, entry, variant)
}

#[test]
fn installed_revocation_discovery_dispatch_rechecks_native_authority_and_preserves_funding() {
    for typed in [false, true] {
        for capacity in [0, 1, 128] {
            let (mut o, mut setup, s) = owner(stream_limits());
            let n = native(&s);
            let stream = o
                .open_source_publication_stream_v1(&s, &stream_facts(&s, &n), &mut setup)
                .unwrap()
                .stream;
            let ds = DiscoveryScopeV1 {
                context: SettingsExecutionContextV1 {
                    profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                    ..s.context.clone()
                },
                policy_epoch: 1,
            };
            let (context, entry, variant) = install(&mut o, &ds, true, &mut setup);
            let target = ObservationStopTargetV3::Existing(
                ObservationStopTargetV2::SourcePublication(&stream),
            );
            let control = SourceReadContextControlV1 {
                scope: &s,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            let port = || {
                ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                    stream: &stream,
                    control: &control,
                })
            };
            let mut authority = TrustedObservationRevocationV1 {
                original_target: target,
                revocation_allowed: Some(false),
            };
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let debt = o.retention_usage();
            for allowed in [false, true, true] {
                authority.revocation_allowed = Some(allowed);
                let cut = prepare(&mut o, &ds, true, &mut setup);
                let f = facts();
                envelope(
                    target,
                    typed,
                    "revoke-observation-grant",
                    "revoke-observation-grant",
                    |e| {
                        let result = o.revoke_discovered_observation_context_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &context,
                                scope: &ds,
                                cut: &cut,
                                entry: &entry,
                                variant: &variant,
                                facts: &f,
                            },
                            e,
                            port(),
                            &authority,
                            &mut r,
                        );
                        assert_eq!(result.is_ok(), allowed);
                    },
                );
                assert_eq!(
                    o.source_publication_usage_v1().unwrap().stopped,
                    usize::from(allowed)
                );
                assert_eq!(o.retention_usage(), debt);
            }
            if capacity == 128 {
                assert!(std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary
                    == Boundary::DiscoveryObservationRevocationDispatch
                    && e.disposition == Disposition::Completed
                    && e.service_time.is_some()));
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
            if capacity == 0 {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}

#[test]
fn revocation_copied_metadata_or_lost_discovery_authority_cannot_close_native_context() {
    for installed in [false, true] {
        let (mut o, mut r, s) = owner(stream_limits());
        let n = native(&s);
        let stream = o
            .open_source_publication_stream_v1(&s, &stream_facts(&s, &n), &mut r)
            .unwrap()
            .stream;
        let sibling = o
            .open_source_publication_stream_v1(&s, &stream_facts(&s, &n), &mut r)
            .unwrap()
            .stream;
        let ds = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..s.context.clone()
            },
            policy_epoch: 1,
        };
        let (context, entry, variant) = install(&mut o, &ds, installed, &mut r);
        let cut = prepare(&mut o, &ds, installed, &mut r);
        let target =
            ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(&stream));
        let authority = TrustedObservationRevocationV1 {
            original_target: target,
            revocation_allowed: Some(true),
        };
        let control = SourceReadContextControlV1 {
            scope: &s,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        let mut f = facts();
        if installed {
            f.discovery_allowed = Some(false);
        }
        let before = o.source_publication_usage_v1().unwrap();
        let other =
            ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(&sibling));
        let other_authority = TrustedObservationRevocationV1 {
            original_target: other,
            revocation_allowed: Some(true),
        };
        envelope(
            other,
            true,
            "revoke-observation-grant",
            "revoke-observation-grant",
            |e| {
                assert_eq!(
                    o.revoke_discovered_observation_context_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &context,
                            scope: &ds,
                            cut: &cut,
                            entry: &entry,
                            variant: &variant,
                            facts: &f,
                        },
                        e,
                        ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                            stream: &stream,
                            control: &control
                        }),
                        &other_authority,
                        &mut r
                    ),
                    Err(DiscoveredObservationRevocationErrorV1::Request(
                        ObservationRevocationIngressErrorV1::Execution(
                            ObservationRevocationErrorV1::Unavailable
                        )
                    ))
                );
            },
        );
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        assert!(!o.discovery_context(&context).unwrap().stopped);
        envelope(
            target,
            true,
            "revoke-observation-grant",
            "revoke-observation-grant",
            |e| {
                assert!(matches!(
                    o.revoke_discovered_observation_context_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &context,
                            scope: &ds,
                            cut: &cut,
                            entry: &entry,
                            variant: &variant,
                            facts: &f,
                        },
                        e,
                        ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                            stream: &stream,
                            control: &control
                        }),
                        &authority,
                        &mut r
                    ),
                    Err(DiscoveredObservationRevocationErrorV1::Discovery(_))
                ));
            },
        );
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        if !installed {
            assert!(!o.discovery_context(&context).unwrap().stopped);
        }
        // A lost ordinary discovery context does not remove protected direct control.
        o.revoke_observation_context_v1(
            target,
            ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                stream: &stream,
                control: &control,
            }),
            &authority,
            &mut r,
        )
        .unwrap();
    }
}
