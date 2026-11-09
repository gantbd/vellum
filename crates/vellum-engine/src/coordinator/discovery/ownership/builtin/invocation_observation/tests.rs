use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::invocation::observation::ingress_tests::request;
use crate::coordinator::invocation::observation::tests::setup_with_retention_budget;
use crate::instrumentation::NativeTimingProfileV1;

fn visibility<const N: usize>() -> NativeBuiltinEntryVisibilityV1<N> {
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
fn prepare(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_invocation_observation_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F06-native-observation-v1",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, false, &visibility::<4>(), |i| {
        o.prepare_discovery_cut_v1(s, i, &facts(), r)
    })
    .unwrap()
}
#[test]
fn installed_f06_queries_join_all_four_native_forms_and_refuse_copied_metadata() {
    for maximum in [0, 1, 256, 4096] {
        assert!(
            workspace_bytes(maximum).unwrap()
                <= super::super::context::workspace_bytes(maximum).unwrap()
        );
    }
    for installed in [false, true] {
        for typed in [false, true] {
            for mode in 0..4 {
                let (mut o, id, c, p, mut v, mut setup) = setup_with_retention_budget(8_388_608);
                let control = InvocationObservationPolicyControlV1 {
                    context: &c,
                    owner_control_allowed: Some(true),
                    retention_released: Some(true),
                    delivery_drained: Some(true),
                };
                let bounds = InvocationObservationLimitsV1 {
                    max_waiters: 1,
                    max_wait_steps: 4,
                    max_issued: 4,
                    max_context_bytes: 2048,
                    max_wait_bytes: 256,
                    max_reference_bytes: 256,
                    max_reply_bytes: 2048,
                    max_control_reply_bytes: 256,
                    max_held_bytes: 16384,
                };
                o.retire_invocation_observation_v1(&mut v, &control, &mut setup)
                    .unwrap();
                let admission = TrustedInvocationPolicyFactsV1 {
                    context: &c,
                    authorized: Some(true),
                    scope: Some(true),
                    continuous: Some(true),
                    live: Some(true),
                    retained: Some(true),
                    transfer_ready: Some(true),
                    resources_ready: Some(true),
                    preparation_reserved: Some(true),
                    registry_ready: Some(true),
                    retention_ready: Some(true),
                };
                let mut v = o
                    .open_invocation_observation_v1(&id, &c, &p, bounds, &admission, &mut setup)
                    .unwrap();
                let context = v.context_id_v1();
                let ds = DiscoveryScopeV1 {
                    context: SettingsExecutionContextV1 {
                        profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                        ..c.clone()
                    },
                    policy_epoch: 1,
                };
                let mut l = limits();
                l.max_context_bytes = 256;
                l.max_entries = 1;
                l.max_variants = 4;
                l.max_row_bytes = 65536;
                l.max_capture_bytes = 131072;
                l.max_packet_bytes = 131072;
                l.max_reply_bytes = 131072;
                l.max_issued = 256;
                l.max_selectors = 64;
                l.metadata.max_bytes = 32768;
                l.metadata.max_nodes = 2048;
                l.metadata.max_depth = 12;
                l.max_held_bytes = l.declared_bytes().unwrap();
                o.configure_discovery_v1(&ds, l, &facts(), &mut setup)
                    .unwrap();
                let cut = prepare(&mut o, &ds, installed, &mut setup);
                let dc = o
                    .acquire_discovery_v1(&ds, &cut, &facts(), &mut setup)
                    .unwrap()
                    .context;
                let cut = prepare(&mut o, &ds, installed, &mut setup);
                let DiscoveryReplyV1::Page { mut entries, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &ds,
                        &cut,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                        &facts(),
                        &mut setup,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                let entry = entries.remove(0).reference;
                let cut = prepare(&mut o, &ds, installed, &mut setup);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &ds,
                        &cut,
                        DiscoveryQueryV1::Inspect { entry: &entry },
                        &facts(),
                        &mut setup,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                assert_eq!(descriptor.variants.len(), 4);
                assert!(descriptor.variants.iter().all(|v| v.schema.is_some()));
                let variants: Vec<_> = descriptor
                    .variants
                    .into_iter()
                    .map(|v| v.reference)
                    .collect();
                let f = TrustedInvocationObservationReleaseV1 {
                    context: &c,
                    authorized: Some(true),
                    scope: Some(true),
                    continuous: Some(true),
                    live: Some(true),
                    retained: Some(true),
                    transfer_ready: Some(true),
                    control_ready: Some(true),
                };
                let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
                    current: &f,
                    ordinary_ready: Some(true),
                    resources_ready: Some(true),
                };
                let wf = TrustedInvocationWaitFactsV1 {
                    observation: &ordinary,
                    wait_capacity: Some(true),
                    wait_clock_valid: Some(true),
                    retention_ready: Some(true),
                    owner_control_allowed: Some(true),
                };
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Query,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                let before = o.retention_usage();
                let mut after = None;
                let mut waiter = None;
                for (index, op) in [
                    (0, "status"),
                    (2, "minimal-control-status"),
                    (3, "bounded-observation-wait"),
                    (1, "retained-outcome"),
                ] {
                    if index == 1 {
                        o.run(&id, &mut setup).unwrap();
                    }
                    if index == 3 && after.is_none() {
                        after = Some(
                            o.query_invocation_observation_v1(
                                &mut v,
                                InvocationObservationQueryV1::Status,
                                &p,
                                &ordinary,
                                &mut setup,
                            )
                            .unwrap()
                            .reference,
                        );
                    }
                    let cut = prepare(&mut o, &ds, installed, &mut setup);
                    let df = facts();
                    let port = match index {
                        2 => InvocationObservationReadPortV1::MinimalControl {
                            current: &p,
                            facts: &f,
                        },
                        3 => InvocationObservationReadPortV1::Wait {
                            current: &p,
                            facts: &wf,
                            clock: InvocationWaitClockV1 { epoch: 9, tick: 0 },
                        },
                        _ => InvocationObservationReadPortV1::Ordinary {
                            current: &p,
                            facts: &ordinary,
                        },
                    };
                    let result = request(
                        &id,
                        &context,
                        &c.profile,
                        typed,
                        op,
                        if index == 3 {
                            Some(InvocationObservationEnvelopeValueV1::After(
                                after.as_ref().unwrap(),
                            ))
                        } else {
                            None
                        },
                        2.0,
                        |e| {
                            o.query_discovered_invocation_observation_v1(
                                NativeDiscoveredCommandRouteV1 {
                                    context: &dc,
                                    scope: &ds,
                                    cut: &cut,
                                    entry: &entry,
                                    variant: &variants[index],
                                    facts: &df,
                                },
                                &mut v,
                                e,
                                port,
                                &mut r,
                            )
                        },
                    );
                    if !installed {
                        assert_eq!(
                            result,
                            Err(DiscoveredInvocationObservationErrorV1::Discovery(
                                Error::Unavailable
                            ))
                        );
                        o.retire_discovery_cut_v1(&cut, &df, &mut setup).unwrap();
                        continue;
                    }
                    match result.unwrap() {
                        InvocationObservationReadReplyV1::Ordinary(v) => {
                            if index == 0 {
                                after = Some(v.reference);
                            } else {
                                assert!(v.outcome.is_some());
                            }
                        }
                        InvocationObservationReadReplyV1::Waiting(w) => {
                            waiter = Some(w);
                            assert_eq!(
                                o.invocation_state(&id, &mut setup),
                                Ok(InvocationState::Pending)
                            );
                        }
                        InvocationObservationReadReplyV1::MinimalControl(_) => (),
                    }
                }
                if let Some(wait) = waiter {
                    assert!(matches!(
                        o.advance_invocation_observation_wait_v1(
                            &mut v,
                            &wait,
                            InvocationWaitEventV1::Poll,
                            &p,
                            &wf,
                            &mut setup
                        )
                        .unwrap(),
                        InvocationWaitReplyV1::Observed(_)
                    ));
                    o.retire_invocation_observation_wait_v1(&mut v, &wait, &control, &mut setup)
                        .unwrap();
                }
                assert_eq!(o.retention_usage(), before);
                let mut completed = 0;
                while let Some(e) = r.pop_front() {
                    if e.boundary == Boundary::DiscoveryInvocationObservationDispatch {
                        assert_eq!(e.service_time.is_some(), mode == 2);
                        completed += usize::from(e.disposition == Disposition::Completed);
                    }
                }
                if mode == 1 || mode == 2 {
                    assert_eq!(completed, if installed { 4 } else { 0 });
                }
                if mode == 3 {
                    assert!(r.dropped() > 0);
                }
                if mode == 0 {
                    assert_eq!(r.dropped(), 0);
                }
            }
        }
    }
}
