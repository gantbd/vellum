use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::invocation::control::arguments::tests::request;
use crate::coordinator::invocation::control::tests::{
    control, queued_with_retention_headroom, stop_requested,
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
            .prepare_native_invocation_cancellation_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F18-invocation-cancellation",
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
fn discovered_cancellation_requires_installed_route_and_current_native_control_without_settlement()
{
    for installed in [false, true] {
        for typed in [false, true] {
            for mode in 0..4 {
                let (mut o, mut setup, c, ticket) = queued_with_retention_headroom(0, 1048576);
                let s = DiscoveryScopeV1 {
                    context: SettingsExecutionContextV1 {
                        profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                        ..c.clone()
                    },
                    policy_epoch: 1,
                };
                let (context, entry, variant) = install(&mut o, &s, installed, &mut setup);
                let generation = o.generation();
                let debt = o.retention_usage();
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
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
                for allowed in [false, true, true] {
                    let cut = prepare(&mut o, &s, installed, &mut setup);
                    let mut f = control(&c);
                    f.owner_control_allowed = Some(allowed);
                    let discovery_facts = facts();
                    request(&ticket, typed, |e| {
                        let result = o.request_discovered_invocation_cancellation_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &context,
                                scope: &s,
                                cut: &cut,
                                entry: &entry,
                                variant: &variant,
                                facts: &discovery_facts,
                            },
                            e,
                            &f,
                            &mut r,
                        );
                        if !installed {
                            assert_eq!(
                                result,
                                Err(DiscoveredInvocationCancellationErrorV1::Discovery(
                                    Error::Unavailable
                                ))
                            );
                        } else if !allowed {
                            assert_eq!(
                                result,
                                Err(DiscoveredInvocationCancellationErrorV1::Request(
                                    InvocationCancellationIngressErrorV1::Execution(
                                        InvocationError::Unavailable
                                    )
                                ))
                            );
                        } else {
                            assert_eq!(result, Ok(()));
                        }
                    });
                    if !installed {
                        o.retire_discovery_cut_v1(&cut, &facts(), &mut setup)
                            .unwrap();
                    }
                    assert_eq!(o.generation(), generation);
                    assert_eq!(o.retention_usage(), debt);
                    assert_eq!(
                        o.invocation_state(&ticket, &mut setup),
                        Ok(InvocationState::Pending)
                    );
                }
                if mode == 0 {
                    assert_eq!(r.queued(), 0);
                    assert_eq!(r.dropped(), 0);
                } else if mode == 3 {
                    assert_eq!(r.pop_front(), Some(sentinel));
                    assert!(r.dropped() > 0);
                } else {
                    let mut dispatch = Vec::new();
                    while let Some(event) = r.pop_front() {
                        if event.boundary == Boundary::DiscoveryInvocationCancellationDispatch {
                            assert_eq!(event.service_time.is_some(), mode == 2);
                            dispatch.push(event.disposition);
                        }
                    }
                    assert_eq!(
                        dispatch,
                        if installed {
                            vec![
                                Disposition::Refused,
                                Disposition::Completed,
                                Disposition::Completed,
                            ]
                        } else {
                            vec![Disposition::Refused; 3]
                        }
                    );
                }
                if installed {
                    assert_eq!(o.run(&ticket, &mut setup), Ok(InvocationState::Finished));
                    assert!(matches!(
                        o.invocation_outcome(&ticket, &mut setup),
                        Ok(InvocationOutcome::StoppedBeforePublication)
                    ));
                } else {
                    assert!(!stop_requested(&o, &ticket));
                }
            }
        }
    }
}

#[test]
fn discovered_cancellation_current_discovery_refusal_does_not_hide_direct_protected_control() {
    let (mut o, mut r, c, ticket) = queued_with_retention_headroom(0, 1048576);
    let s = DiscoveryScopeV1 {
        context: SettingsExecutionContextV1 {
            profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
            ..c.clone()
        },
        policy_epoch: 1,
    };
    let (context, entry, variant) = install(&mut o, &s, true, &mut r);
    let cut = prepare(&mut o, &s, true, &mut r);
    let mut f = facts();
    f.interpretation_current = Some(false);
    request(&ticket, true, |e| {
        assert!(matches!(
            o.request_discovered_invocation_cancellation_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &context,
                    scope: &s,
                    cut: &cut,
                    entry: &entry,
                    variant: &variant,
                    facts: &f
                },
                e,
                &control(&c),
                &mut r
            ),
            Err(DiscoveredInvocationCancellationErrorV1::Discovery(_))
        ));
        assert!(!stop_requested(&o, &ticket));
        assert_eq!(
            o.request_decoded_invocation_cancellation_v1(e, &control(&c), &mut r),
            Ok(())
        );
    });
}
