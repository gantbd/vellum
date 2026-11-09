use super::*;
mod shutdown;
#[test]
fn versioned_stop_binds_discovery_context_before_current_control() {
    for typed in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        let s = scope();
        let f = facts();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        o.configure_discovery_v1(&s, limits(), &f, &mut r).unwrap();
        let cut = o
            .prepare_discovery_cut_v1(
                &s,
                DiscoveryCutInputV1 {
                    entries: &[],
                    target: None,
                    advice_present: false,
                },
                &f,
                &mut r,
            )
            .unwrap();
        let a = o
            .acquire_discovery_v1(&s, &cut, &f, &mut r)
            .unwrap()
            .context;
        let cut = o
            .prepare_discovery_cut_v1(
                &s,
                DiscoveryCutInputV1 {
                    entries: &[],
                    target: None,
                    advice_present: false,
                },
                &f,
                &mut r,
            )
            .unwrap();
        let b = o
            .acquire_discovery_v1(&s, &cut, &f, &mut r)
            .unwrap()
            .context;
        let debt = o.retention_usage();
        for target in [&b, &a, &a] {
            let result = crate::coordinator::observation_control::request(
                ObservationStopTargetV2::Discovery(target),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v2(
                        DecodedObservationStopRequestV2 {
                            envelope,
                            native: ObservationStopPortV2::Existing(
                                ObservationStopPortV1::Discovery {
                                    context: &a,
                                    scope: &s,
                                    facts: &f,
                                },
                            ),
                        },
                        &mut r,
                    )
                },
            );
            assert_eq!(result.is_ok(), target == &a);
            assert_eq!(o.discovery_context(&a).unwrap().stopped, target == &a);
            assert!(!o.discovery_context(&b).unwrap().stopped);
            assert_eq!(o.retention_usage(), debt);
        }
    }
}
#[test]
fn heterogeneous_stop_does_not_promote_discovery_local_fault_to_editing_fault() {
    let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
    let s = scope();
    let f = facts();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_discovery_v1(&s, limits(), &f, &mut r).unwrap();
    let cut = o
        .prepare_discovery_cut_v1(
            &s,
            DiscoveryCutInputV1 {
                entries: &[],
                target: None,
                advice_present: false,
            },
            &f,
            &mut r,
        )
        .unwrap();
    let context = o
        .acquire_discovery_v1(&s, &cut, &f, &mut r)
        .unwrap()
        .context;
    let debt = o.retention_usage();
    o.discovery_store_mut().unwrap().faulted = true;
    assert_eq!(
        o.stop_observation_facade_v1(
            ObservationStopPortV1::Discovery {
                context: &context,
                scope: &s,
                facts: &f
            },
            &mut r
        ),
        Ok(())
    );
    assert!(!o.is_faulted());
    assert!(o.discovery_store().unwrap().faulted);
    assert_eq!(o.retention_usage(), debt);
}
#[test]
fn heterogeneous_discovery_stop_checks_original_scope_and_never_refunds_or_heals() {
    for allowed in [None, Some(false), Some(true)] {
        for faulted in [false, true] {
            let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
            let s = scope();
            let mut f = facts();
            o.configure_retention_budget(1048576, &mut r).unwrap();
            o.configure_discovery_v1(&s, limits(), &f, &mut r).unwrap();
            let cut = o
                .prepare_discovery_cut_v1(
                    &s,
                    DiscoveryCutInputV1 {
                        entries: &[],
                        target: None,
                        advice_present: false,
                    },
                    &f,
                    &mut r,
                )
                .unwrap();
            let context = o
                .acquire_discovery_v1(&s, &cut, &f, &mut r)
                .unwrap()
                .context;
            let debt = o.retention_usage();
            let generation = o.generation();
            f.owner_control_allowed = allowed;
            f.authorized = None;
            f.retention_released = None;
            f.delivery_drained = None;
            o.faulted = faulted;
            let mut wrong = s.clone();
            wrong.policy_epoch += 1;
            assert!(
                o.stop_observation_facade_v1(
                    ObservationStopPortV1::Discovery {
                        context: &context,
                        scope: &wrong,
                        facts: &f
                    },
                    &mut r
                )
                .is_err()
            );
            assert!(!o.discovery_context(&context).unwrap().stopped);
            let result = o.stop_observation_facade_v1(
                ObservationStopPortV1::Discovery {
                    context: &context,
                    scope: &s,
                    facts: &f,
                },
                &mut r,
            );
            assert_eq!(result.is_ok(), allowed == Some(true));
            assert_eq!(
                o.discovery_context(&context).unwrap().stopped,
                allowed == Some(true)
            );
            assert_eq!(o.is_faulted(), faulted);
            assert_eq!(o.retention_usage(), debt);
            assert_eq!(o.generation(), generation);
        }
    }
}
pub(super) fn scope() -> DiscoveryScopeV1 {
    let mut context = crate::coordinator::serialization::tests::context();
    context.profile = "owned-contract-discovery/1".into();
    DiscoveryScopeV1 {
        context,
        policy_epoch: 1,
    }
}
pub(super) fn limits() -> DiscoveryLimitsV1 {
    DiscoveryLimitsV1 {
        max_contexts: 2,
        max_preparations: 2,
        max_entries: 4,
        max_variants: 4,
        max_selectors: 8,
        max_issued: 16,
        max_page_rows: 2,
        max_root_bytes: 1048576,
        max_packet_bytes: 16384,
        max_context_bytes: 4096,
        max_capture_bytes: 16384,
        max_row_bytes: 4096,
        max_selector_bytes: 512,
        max_reply_bytes: 8192,
        metadata: CaptureLimitsV1 {
            max_bytes: 4096,
            max_nodes: 128,
            max_depth: 8,
        },
        // One owner prepays all supported profile source/matcher staging.
        max_held_bytes: 524288,
    }
}
pub(super) fn facts() -> TrustedDiscoveryFactsV1 {
    TrustedDiscoveryFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        discovery_allowed: Some(true),
        interpretation_current: Some(true),
        accepted_registry_attested: Some(true),
        schema_descriptions_attested: Some(true),
        visibility_attested: Some(true),
        visibility_complete: Some(true),
        projection_order_attested: Some(true),
        outside_publication_guard: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        retention_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        registry_ready: Some(true),
        reply_ready: Some(true),
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
        ..Default::default()
    }
}
#[test]
fn exact_native_parent_packet_scope_capture_reply_and_shared_funding_boundaries() {
    for field in 0..6 {
        for delta in [-1, 0, 1] {
            let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
            let c = scope();
            let g = facts();
            let mut l = limits();
            match field {
                0 => {
                    l.max_held_bytes = l
                        .declared_bytes()
                        .unwrap()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                2 => {
                    l.max_packet_bytes = (size_of::<Packet>() + scope_bytes(&c).unwrap())
                        .checked_add_signed(delta)
                        .unwrap()
                }
                3 => {
                    l.max_context_bytes =
                        scope_bytes(&c).unwrap().checked_add_signed(delta).unwrap()
                }
                4 => {
                    l.max_capture_bytes = (size_of::<Context>() + scope_bytes(&c).unwrap())
                        .checked_add_signed(delta)
                        .unwrap()
                }
                5 => {
                    l.max_reply_bytes = size_of::<DiscoveryAcquisitionV1>()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                _ => {}
            }
            let budget = if field == 1 {
                l.max_held_bytes.checked_add_signed(delta).unwrap()
            } else {
                1048576
            };
            o.configure_retention_budget(budget, &mut r).unwrap();
            let configured = o.configure_discovery_v1(&c, l, &g, &mut r);
            if delta < 0 && (field < 2 || field == 3) {
                assert_eq!(
                    configured,
                    Err(if field == 3 {
                        Error::InvalidRequest
                    } else {
                        Error::Limit
                    })
                );
                assert_eq!(o.retention_usage().reserved, 0);
                continue;
            }
            configured.unwrap();
            let debt = o.retention_usage();
            let prepared = o.prepare_discovery_cut_v1(
                &c,
                DiscoveryCutInputV1 {
                    entries: &[],
                    target: None,
                    advice_present: false,
                },
                &g,
                &mut r,
            );
            if field == 2 && delta < 0 {
                assert_eq!(prepared, Err(Error::Limit));
                assert_eq!(o.discovery_usage_v1().unwrap().preparations, 0);
            } else {
                let packet = prepared.unwrap();
                let acquired = o.acquire_discovery_v1(&c, &packet, &g, &mut r);
                if delta < 0 && field >= 4 {
                    assert_eq!(acquired, Err(Error::Limit));
                    assert_eq!(o.discovery_usage_v1().unwrap().issued_lifetime, 0);
                } else {
                    acquired.unwrap();
                }
            }
            assert_eq!(o.retention_usage(), debt);
        }
    }
}
#[test]
fn discovery_integrity_fault_is_local_and_protected_handles_do_not_reset_uncertain_debt() {
    let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
    let c = scope();
    let g = facts();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_discovery_v1(&c, limits(), &g, &mut r).unwrap();
    let packet = o
        .prepare_discovery_cut_v1(
            &c,
            DiscoveryCutInputV1 {
                entries: &[],
                target: None,
                advice_present: false,
            },
            &g,
            &mut r,
        )
        .unwrap();
    let context = o
        .acquire_discovery_v1(&c, &packet, &g, &mut r)
        .unwrap()
        .context;
    let debt = o.retention_usage();
    o.discovery_store_mut().unwrap().issued = limits().max_issued + 1;
    assert_eq!(
        o.prepare_discovery_cut_v1(
            &c,
            DiscoveryCutInputV1 {
                entries: &[],
                target: None,
                advice_present: false
            },
            &g,
            &mut r
        ),
        Err(Error::Faulted)
    );
    assert!(!o.is_faulted());
    assert!(o.discovery_store().unwrap().faulted);
    o.stop_discovery_context_v1(&context, &c, &g, &mut r)
        .unwrap();
    o.retire_discovery_context_v1(&context, &g, &mut r).unwrap();
    o.stop_discovery_admission_v1(&g, &mut r).unwrap();
    assert_eq!(
        o.retire_discovery_registry_v1(&g, &mut r),
        Err(Error::Faulted)
    );
    assert_eq!(o.retention_usage(), debt);
}
