use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
#[path = "discovery_behaviors/annotation_dispatch.rs"]
mod annotation_dispatch;
#[path = "discovery_behaviors/builtin.rs"]
mod builtin;
#[path = "support/resource_catalog_fixture.rs"]
#[allow(dead_code)]
mod catalog_support;
#[path = "discovery_behaviors/context.rs"]
mod context;
#[path = "discovery_behaviors/decoder.rs"]
mod decoder;
#[path = "discovery_behaviors/editing.rs"]
mod editing;
#[path = "discovery_behaviors/lifecycle.rs"]
mod lifecycle;
#[path = "support/invocation_observation_fixture.rs"]
#[allow(dead_code)]
mod observation_support;
#[path = "discovery_behaviors/recognition.rs"]
mod recognition;
#[path = "discovery_behaviors/reversal_dispatch.rs"]
mod reversal_dispatch;
#[path = "discovery_behaviors/self_description.rs"]
mod self_description;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
fn scope() -> DiscoveryScopeV1 {
    DiscoveryScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "authority".into(),
            engine: "engine".into(),
            recipient: "recipient".into(),
            purpose: "discover".into(),
            destination: "client".into(),
            profile: "owned-contract-discovery/1".into(),
            continuity: "continuity".into(),
        },
        policy_epoch: 1,
    }
}
fn limits() -> DiscoveryLimitsV1 {
    DiscoveryLimitsV1 {
        max_contexts: 4,
        max_preparations: 4,
        max_entries: 8,
        max_variants: 8,
        max_selectors: 32,
        max_issued: 64,
        max_page_rows: 4,
        max_root_bytes: 1048576,
        max_packet_bytes: 65536,
        max_context_bytes: 4096,
        max_capture_bytes: 32768,
        max_row_bytes: 8192,
        max_selector_bytes: 1024,
        max_reply_bytes: 16384,
        metadata: CaptureLimitsV1 {
            max_bytes: 8192,
            max_nodes: 128,
            max_depth: 8,
        },
        max_held_bytes: 1048576,
    }
}
fn facts() -> TrustedDiscoveryFactsV1 {
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
        target_binding_attested: Some(true),
        target_equality_observable: Some(true),
        advice_attested: Some(true),
        target_current: Some(true),
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn setup() -> support::Fixture {
    let mut f = support::Fixture::new();
    f.owner
        .configure_retention_budget(4194304, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_discovery_v1(&scope(), limits(), &facts(), &mut f.recorder)
        .unwrap();
    f
}
fn cut(f: &mut support::Fixture, visible: bool, schema: bool) -> DiscoveryCutIdV1 {
    metadata(visible, schema, |input| {
        f.owner
            .prepare_discovery_cut_v1(&scope(), input, &facts(), &mut f.recorder)
    })
    .unwrap()
}
fn acquire(f: &mut support::Fixture, visible: bool, schema: bool) -> DiscoveryAcquisitionV1 {
    let p = cut(f, visible, schema);
    f.owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap()
}
fn query(
    f: &mut support::Fixture,
    c: &DiscoveryContextIdV1,
    q: DiscoveryQueryV1<'_>,
    schema: bool,
) -> Result<DiscoveryReplyV1, DiscoveryErrorV1> {
    let p = cut(f, true, schema);
    f.owner
        .query_discovery_v1(c, &scope(), &p, q, &facts(), &mut f.recorder)
}
fn entry(f: &mut support::Fixture, c: &DiscoveryContextIdV1, schema: bool) -> DiscoveryEntryIdV1 {
    let DiscoveryReplyV1::Page {
        context,
        coverage,
        entries,
        continuation,
    } = query(
        f,
        c,
        DiscoveryQueryV1::List {
            limit: 1,
            continuation: None,
        },
        schema,
    )
    .unwrap()
    else {
        panic!("page");
    };
    assert_eq!(context, *c);
    assert_eq!(coverage, DiscoveryCoverageV1::CapturedPermitted);
    assert!(continuation.is_none());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].version, 1);
    entries[0].reference.clone()
}
#[test]
fn unsupported_discovery_profiles_do_not_install_an_owner_or_borrow_a_later_schema_profile() {
    for profile in ["unknown/1", "owned-contract-discovery/13", ""] {
        let mut f = support::Fixture::new();
        f.owner
            .configure_retention_budget(4194304, &mut f.recorder)
            .unwrap();
        let mut c = scope();
        c.context.profile = profile.into();
        assert_eq!(
            f.owner
                .configure_discovery_v1(&c, limits(), &facts(), &mut f.recorder),
            Err(DiscoveryErrorV1::InvalidRequest)
        );
        assert_eq!(f.owner.retention_usage().reserved, 0);
    }
}
fn metadata<T>(visible: bool, schema: bool, run: impl FnOnce(DiscoveryCutInputV1<'_>) -> T) -> T {
    let fields = [("operation", D::Text("status"))];
    let input = [
        ("contract", D::Text("example/1")),
        ("variant", D::Text("status")),
        ("fields", D::Object(&fields)),
    ];
    let presentation = [("title", D::Text("literal untrusted metadata"))];
    let variants = [DiscoveryVariantInputV1 {
        id: "status",
        input: D::Object(&input),
        outcomes: D::Text("bounded"),
        requirements: D::Null,
        limits: D::Null,
        cancellation: D::Text("observer only"),
        order: visible.then_some(0),
        enabled: Some(false),
    }];
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority: "authority",
            id: "example",
            incarnation: 1,
        },
        contract: "example/1",
        version: 1,
        kind: DiscoveryExecutionKindV1::Query,
        variants: &variants,
        presentation: D::Object(&presentation),
        order: visible.then_some(0),
        schema_visible: visible && schema,
        presentation_visible: visible,
        advice_visible: visible,
    }];
    run(DiscoveryCutInputV1 {
        entries: &entries,
        target: Some(DiscoveryTargetInputV1 {
            identity: "target",
            basis: D::Number(1.0),
        }),
        advice_present: true,
    })
}
#[test]
fn native_discovery_context_consumes_a_root_bound_cut_without_publishing_or_registering_handlers() {
    let mut f = support::Fixture::new();
    f.owner
        .configure_retention_budget(4194304, &mut f.recorder)
        .unwrap();
    let c = scope();
    let g = facts();
    let before = f.owner.generation();
    let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    f.owner
        .configure_discovery_v1(&c, limits(), &g, &mut f.recorder)
        .unwrap();
    let cut = f
        .owner
        .prepare_discovery_cut_v1(
            &c,
            DiscoveryCutInputV1 {
                entries: &[],
                target: None,
                advice_present: false,
            },
            &g,
            &mut f.recorder,
        )
        .unwrap();
    let acquired = f
        .owner
        .acquire_discovery_v1(&c, &cut, &g, &mut f.recorder)
        .unwrap();
    assert_eq!(acquired.target, None);
    assert!(
        f.owner
            .acquire_discovery_v1(&c, &cut, &g, &mut f.recorder)
            .is_err()
    );
    assert_eq!(f.owner.generation(), before);
    assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
}
#[test]
fn preparation_requires_every_independent_authority_and_attestation_without_spending_capacity() {
    let mut f = setup();
    let c = scope();
    let before = f.owner.discovery_usage_v1().unwrap();
    for field in 0..15 {
        for denied in [None, Some(false)] {
            let mut g = facts();
            let slot = match field {
                0 => &mut g.authorized,
                1 => &mut g.scope,
                2 => &mut g.continuous,
                3 => &mut g.live,
                4 => &mut g.not_stopped,
                5 => &mut g.discovery_allowed,
                6 => &mut g.interpretation_current,
                7 => &mut g.accepted_registry_attested,
                8 => &mut g.schema_descriptions_attested,
                9 => &mut g.visibility_attested,
                10 => &mut g.visibility_complete,
                11 => &mut g.projection_order_attested,
                12 => &mut g.outside_publication_guard,
                13 => &mut g.resources_ready,
                _ => &mut g.preparation_reserved,
            };
            *slot = denied;
            assert_eq!(
                metadata(true, true, |input| f.owner.prepare_discovery_cut_v1(
                    &c,
                    input,
                    &g,
                    &mut f.recorder
                )),
                Err(DiscoveryErrorV1::Unavailable)
            );
            assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
        }
    }
}
#[test]
fn acquisition_checks_current_grants_complete_scope_and_native_publication_before_consumption() {
    let mut f = setup();
    let c = scope();
    let g = facts();
    let cut = metadata(true, true, |input| {
        f.owner
            .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
    })
    .unwrap();
    for field in 0..6 {
        let mut g = facts();
        let slot = match field {
            0 => &mut g.outside_publication_guard,
            1 => &mut g.retention_ready,
            2 => &mut g.transfer_ready,
            3 => &mut g.registry_ready,
            4 => &mut g.resources_ready,
            _ => &mut g.reply_ready,
        };
        *slot = None;
        assert_eq!(
            f.owner.acquire_discovery_v1(&c, &cut, &g, &mut f.recorder),
            Err(DiscoveryErrorV1::Unavailable)
        );
    }
    for field in 0..8 {
        let mut wrong = c.clone();
        match field {
            0 => wrong.context.authority = "other".into(),
            1 => wrong.context.engine = "other".into(),
            2 => wrong.context.recipient = "other".into(),
            3 => wrong.context.purpose = "other".into(),
            4 => wrong.context.destination = "other".into(),
            5 => wrong.context.profile = "other".into(),
            6 => wrong.context.continuity = "other".into(),
            _ => wrong.policy_epoch += 1,
        }
        assert_eq!(
            f.owner
                .acquire_discovery_v1(&wrong, &cut, &g, &mut f.recorder),
            Err(DiscoveryErrorV1::Unavailable)
        );
    }
    assert_eq!(f.owner.discovery_usage_v1().unwrap().preparations, 1);
    f.owner
        .create(b"unrelated publication", &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.acquire_discovery_v1(&c, &cut, &g, &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
    f.owner
        .retire_discovery_cut_v1(&cut, &g, &mut f.recorder)
        .unwrap();
    let fresh = metadata(true, true, |input| {
        f.owner
            .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
    })
    .unwrap();
    f.owner
        .acquire_discovery_v1(&c, &fresh, &g, &mut f.recorder)
        .unwrap();
}
#[test]
fn captured_visibility_and_target_observability_do_not_infer_grants_or_advice_truth() {
    for visible in [false, true] {
        for schema in [false, true] {
            for target_known in [false, true] {
                let mut f = setup();
                let c = scope();
                let mut g = facts();
                g.target_equality_observable = target_known.then_some(true);
                g.advice_attested = None;
                let cut = metadata(visible, schema, |input| {
                    f.owner
                        .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
                })
                .unwrap();
                assert_eq!(f.owner.discovery_usage_v1().unwrap().prepared_advice, 0);
                let out = f
                    .owner
                    .acquire_discovery_v1(&c, &cut, &g, &mut f.recorder)
                    .unwrap();
                assert_eq!(out.target.is_some(), target_known);
                let captured = f.owner.discovery_context_usage_v1(&out.context).unwrap();
                assert_eq!(
                    (captured.entries, captured.variants),
                    (usize::from(visible), usize::from(visible))
                );
            }
        }
    }
}
#[test]
fn protected_stop_and_drain_preserve_spent_issuance_and_prevent_registry_replacement() {
    let mut f = setup();
    let c = scope();
    let g = facts();
    let funding = f.owner.retention_usage();
    let cut = metadata(true, true, |input| {
        f.owner
            .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
    })
    .unwrap();
    let out = f
        .owner
        .acquire_discovery_v1(&c, &cut, &g, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .retire_discovery_context_v1(&out.context, &g, &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
    let mut wrong = c.clone();
    wrong.context.recipient = "other".into();
    assert_eq!(
        f.owner
            .stop_discovery_context_v1(&out.context, &wrong, &g, &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
    f.owner
        .stop_discovery_context_v1(&out.context, &c, &g, &mut f.recorder)
        .unwrap();
    let stopped = f.owner.discovery_context_usage_v1(&out.context).unwrap();
    assert!(stopped.stopped);
    assert_eq!((stopped.entries, stopped.variants), (0, 0));
    let mut denied = facts();
    denied.delivery_drained = None;
    assert_eq!(
        f.owner
            .retire_discovery_context_v1(&out.context, &denied, &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
    f.owner
        .retire_discovery_context_v1(&out.context, &g, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.discovery_usage_v1().unwrap().issued_lifetime, 2);
    assert_eq!(f.owner.retention_usage(), funding);
    f.owner
        .stop_discovery_admission_v1(&g, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_registry_v1(&g, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
    assert_eq!(
        f.owner
            .configure_discovery_v1(&c, limits(), &g, &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
}
#[test]
fn live_packet_and_context_limits_refuse_without_reusing_old_opaque_markers() {
    let mut f = setup();
    let c = scope();
    let g = facts();
    let mut packets = Vec::new();
    for _ in 0..4 {
        packets.push(
            metadata(true, true, |input| {
                f.owner
                    .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
            })
            .unwrap(),
        );
    }
    assert_eq!(
        metadata(true, true, |input| f.owner.prepare_discovery_cut_v1(
            &c,
            input,
            &g,
            &mut f.recorder
        )),
        Err(DiscoveryErrorV1::Limit)
    );
    let old = packets.pop().unwrap();
    f.owner
        .retire_discovery_cut_v1(&old, &g, &mut f.recorder)
        .unwrap();
    let new = metadata(true, true, |input| {
        f.owner
            .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
    })
    .unwrap();
    assert_ne!(old, new);
    assert_eq!(
        f.owner.acquire_discovery_v1(&c, &old, &g, &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
    packets.push(new);
    for packet in packets {
        f.owner
            .acquire_discovery_v1(&c, &packet, &g, &mut f.recorder)
            .unwrap();
    }
    let packet = metadata(true, true, |input| {
        f.owner
            .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
    })
    .unwrap();
    assert_eq!(
        f.owner
            .acquire_discovery_v1(&c, &packet, &g, &mut f.recorder),
        Err(DiscoveryErrorV1::Limit)
    );
    assert_eq!(f.owner.discovery_usage_v1().unwrap().preparations, 0);
}
#[test]
fn descriptor_text_is_literal_and_capture_off_zero_or_available_changes_no_native_outcome() {
    for capacity in [None, Some(0), Some(128)] {
        let mut f = setup();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = scope();
        let g = facts();
        let before = f.owner.generation();
        let cut = metadata(true, true, |input| {
            f.owner
                .prepare_discovery_cut_v1(&c, input, &g, &mut f.recorder)
        })
        .unwrap();
        let out = f
            .owner
            .acquire_discovery_v1(&c, &cut, &g, &mut f.recorder)
            .unwrap();
        f.owner
            .stop_discovery_context_v1(&out.context, &c, &g, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_discovery_context_v1(&out.context, &g, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.generation(), before);
        let mut events = String::new();
        while let Some(e) = f.recorder.pop_front() {
            events.push_str(&format!("{e:?}"));
        }
        assert!(!events.contains("literal untrusted metadata") && !events.contains("recipient"));
        if capacity == Some(128) {
            assert!(
                events.contains("DiscoveryPreparation")
                    && events.contains("DiscoveryAcquisition")
                    && events.contains("DiscoveryContextRetirement")
            );
        }
    }
}

#[test]
fn native_discovery_lists_inspects_and_returns_false_advice_without_effects() {
    for capacity in [None, Some(0), Some(128)] {
        let mut f = setup();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = acquire(&mut f, true, true);
        let generation = f.owner.generation();
        let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
        let funding = f.owner.retention_usage();
        let e = entry(&mut f, &c.context, true);
        let DiscoveryReplyV1::Descriptor {
            context,
            descriptor: d,
        } = query(
            &mut f,
            &c.context,
            DiscoveryQueryV1::Inspect { entry: &e },
            true,
        )
        .unwrap()
        else {
            panic!("descriptor");
        };
        assert_eq!(context, c.context);
        assert_eq!(d.reference, e);
        assert_eq!(d.variants.len(), 1);
        let schema = d.variants[0].schema.as_ref().unwrap();
        let vellum_engine::capture::CapturedValueV1::Object(fields) = schema.root() else {
            panic!("schema object");
        };
        assert_eq!(
            fields.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            [
                "input",
                "outcomes",
                "requirements",
                "limits",
                "cancellation"
            ]
        );
        assert!(d.presentation.is_some());
        let v = &d.variants[0].reference;
        let target = c.target.as_ref().unwrap();
        let DiscoveryReplyV1::Advice {
            context,
            entry,
            variant,
            target: t,
            enabled,
        } = query(
            &mut f,
            &c.context,
            DiscoveryQueryV1::AdvisoryEnablement {
                entry: &e,
                variant: v,
                target,
            },
            true,
        )
        .unwrap()
        else {
            panic!("advice");
        };
        assert_eq!(
            (context, entry, variant, t, enabled),
            (
                c.context.clone(),
                e.clone(),
                v.clone(),
                target.clone(),
                false
            )
        );
        let usage = f.owner.discovery_usage_v1().unwrap();
        assert_eq!((usage.selectors, usage.issued_lifetime), (2, 4));
        assert_eq!(f.owner.retention_usage(), funding);
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
        let mut diagnostic = String::new();
        while let Some(event) = f.recorder.pop_front() {
            diagnostic.push_str(&format!("{event:?}"));
        }
        if capacity == Some(128) {
            assert!(diagnostic.contains("DiscoveryQuery"));
            assert!(diagnostic.contains("DiscoveryResultValidation"));
            assert!(diagnostic.contains("DiscoveryRootCurrentness"));
        }
        for secret in [
            "literal untrusted metadata",
            "status",
            "recipient",
            "example/1",
        ] {
            assert!(!diagnostic.contains(secret));
        }
    }
}

#[test]
fn discovery_never_retrofits_hidden_facets_or_widens_an_empty_capture() {
    let mut f = setup();
    let c = acquire(&mut f, true, false);
    let e = entry(&mut f, &c.context, true);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = query(
        &mut f,
        &c.context,
        DiscoveryQueryV1::Inspect { entry: &e },
        true,
    )
    .unwrap() else {
        panic!("descriptor");
    };
    assert!(descriptor.variants[0].schema.is_none());
    let hidden = acquire(&mut f, false, false);
    let DiscoveryReplyV1::Page {
        entries,
        continuation,
        ..
    } = query(
        &mut f,
        &hidden.context,
        DiscoveryQueryV1::List {
            limit: 4,
            continuation: None,
        },
        true,
    )
    .unwrap()
    else {
        panic!("page");
    };
    assert!(entries.is_empty());
    assert!(continuation.is_none());
    assert!(
        !f.owner
            .discovery_context_usage_v1(&hidden.context)
            .unwrap()
            .stopped
    );
}

#[test]
fn disclosure_narrowing_stops_and_clears_selectors_but_not_spent_issuance_or_funding() {
    for hide_row in [false, true] {
        let mut f = setup();
        let c = acquire(&mut f, true, true);
        let e = entry(&mut f, &c.context, true);
        let before = f.owner.discovery_usage_v1().unwrap();
        let funding = f.owner.retention_usage();
        let p = cut(&mut f, !hide_row, false);
        assert!(matches!(
            f.owner.query_discovery_v1(
                &c.context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &e },
                &facts(),
                &mut f.recorder
            ),
            Err(DiscoveryErrorV1::Unavailable)
        ));
        assert!(
            f.owner
                .discovery_context_usage_v1(&c.context)
                .unwrap()
                .stopped
        );
        let after = f.owner.discovery_usage_v1().unwrap();
        assert_eq!(
            (after.selectors, after.issued_lifetime),
            (0, before.issued_lifetime)
        );
        assert_eq!(f.owner.retention_usage(), funding);
        assert!(
            query(
                &mut f,
                &c.context,
                DiscoveryQueryV1::Inspect { entry: &e },
                true
            )
            .is_err()
        );
    }
}

#[test]
fn explicit_loss_is_absorbing_unknown_withholds_and_foreign_scope_cannot_apply_loss() {
    for field in 0..8 {
        for value in [None, Some(false)] {
            let mut f = setup();
            let c = acquire(&mut f, true, true);
            let p = cut(&mut f, true, true);
            let mut g = facts();
            *match field {
                0 => &mut g.authorized,
                1 => &mut g.scope,
                2 => &mut g.continuous,
                3 => &mut g.live,
                4 => &mut g.not_stopped,
                5 => &mut g.discovery_allowed,
                6 => &mut g.interpretation_current,
                _ => &mut g.retained,
            } = value;
            let mut foreign = scope();
            foreign.context.recipient = "foreign".into();
            assert!(
                f.owner
                    .query_discovery_v1(
                        &c.context,
                        &foreign,
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None
                        },
                        &g,
                        &mut f.recorder
                    )
                    .is_err()
            );
            assert!(
                !f.owner
                    .discovery_context_usage_v1(&c.context)
                    .unwrap()
                    .stopped
            );
            assert!(
                f.owner
                    .query_discovery_v1(
                        &c.context,
                        &scope(),
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None
                        },
                        &g,
                        &mut f.recorder
                    )
                    .is_err()
            );
            assert_eq!(
                f.owner
                    .discovery_context_usage_v1(&c.context)
                    .unwrap()
                    .stopped,
                value == Some(false)
            );
            assert_eq!(f.owner.discovery_usage_v1().unwrap().preparations, 1);
            if value.is_none() {
                assert!(
                    f.owner
                        .query_discovery_v1(
                            &c.context,
                            &scope(),
                            &p,
                            DiscoveryQueryV1::List {
                                limit: 1,
                                continuation: None
                            },
                            &facts(),
                            &mut f.recorder
                        )
                        .is_ok()
                );
            }
        }
    }
}

#[test]
fn release_readiness_is_strict_without_stopping_or_consuming_the_cut() {
    for field in 0..4 {
        for value in [None, Some(false)] {
            let mut f = setup();
            let c = acquire(&mut f, true, true);
            let p = cut(&mut f, true, true);
            let mut g = facts();
            *match field {
                0 => &mut g.transfer_ready,
                1 => &mut g.reply_ready,
                2 => &mut g.resources_ready,
                _ => &mut g.outside_publication_guard,
            } = value;
            assert!(
                f.owner
                    .query_discovery_v1(
                        &c.context,
                        &scope(),
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None
                        },
                        &g,
                        &mut f.recorder
                    )
                    .is_err()
            );
            assert!(
                !f.owner
                    .discovery_context_usage_v1(&c.context)
                    .unwrap()
                    .stopped
            );
            assert!(
                f.owner
                    .query_discovery_v1(
                        &c.context,
                        &scope(),
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None
                        },
                        &facts(),
                        &mut f.recorder
                    )
                    .is_ok()
            );
        }
    }
}

#[test]
fn discovery_queries_require_original_roles_fresh_cuts_and_reuse_issued_selectors() {
    let mut f = setup();
    let a = acquire(&mut f, true, true);
    let b = acquire(&mut f, true, true);
    let e = entry(&mut f, &a.context, true);
    let issued = f.owner.discovery_usage_v1().unwrap().issued_lifetime;
    assert_eq!(entry(&mut f, &a.context, true), e);
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().issued_lifetime,
        issued
    );
    assert!(
        query(
            &mut f,
            &b.context,
            DiscoveryQueryV1::Inspect { entry: &e },
            true
        )
        .is_err()
    );
    let p = cut(&mut f, true, true);
    assert!(
        f.owner
            .query_discovery_v1(
                &a.context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &e },
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
    assert!(
        f.owner
            .query_discovery_v1(
                &a.context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &e },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    let p = cut(&mut f, true, true);
    f.owner.create(b"publication", &mut f.recorder).unwrap();
    assert!(
        f.owner
            .query_discovery_v1(
                &a.context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry: &e },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert!(
        !f.owner
            .discovery_context_usage_v1(&a.context)
            .unwrap()
            .stopped
    );
    assert!(
        query(
            &mut f,
            &a.context,
            DiscoveryQueryV1::Inspect { entry: &e },
            true
        )
        .is_ok()
    );
}

#[test]
fn stopping_new_context_admissions_does_not_stop_queries_or_cut_preparation_for_existing_contexts()
{
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    f.owner
        .stop_discovery_admission_v1(&facts(), &mut f.recorder)
        .unwrap();
    let p = cut(&mut f, true, true);
    assert!(
        f.owner
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
            .is_err()
    );
    assert!(
        f.owner
            .query_discovery_v1(
                &c.context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
}

fn paged_cut(f: &mut support::Fixture, second_visible: bool, title: &str) -> DiscoveryCutIdV1 {
    metadata(true, true, |base| {
        let base = &base.entries[0];
        let presentation = [("title", D::Text(title))];
        let entries: [_; 3] = std::array::from_fn(|i| DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority: "authority",
                id: ["alpha", "private hidden command", "beta"][i],
                incarnation: 1,
            },
            contract: base.contract,
            version: base.version,
            kind: base.kind,
            variants: base.variants,
            presentation: D::Object(&presentation),
            order: match i {
                0 => Some(0),
                2 if second_visible => Some(1),
                _ => None,
            },
            schema_visible: i == 0 || (i == 2 && second_visible),
            presentation_visible: i == 0 || (i == 2 && second_visible),
            advice_visible: i == 0 || (i == 2 && second_visible),
        });
        // Hidden entries must also have hidden variants.
        let hidden_variants = [DiscoveryVariantInputV1 {
            id: "status",
            input: base.variants[0].input,
            outcomes: base.variants[0].outcomes,
            requirements: D::Null,
            limits: D::Null,
            cancellation: base.variants[0].cancellation,
            order: None,
            enabled: Some(false),
        }];
        let mut entries = entries;
        entries[1].variants = &hidden_variants;
        if !second_visible {
            entries[2].variants = &hidden_variants;
        }
        f.owner.prepare_discovery_cut_v1(
            &scope(),
            DiscoveryCutInputV1 {
                entries: &entries,
                target: None,
                advice_present: true,
            },
            &facts(),
            &mut f.recorder,
        )
    })
    .unwrap()
}

#[test]
fn pages_are_dense_context_scoped_and_cursor_limit_bound_with_stable_selectors() {
    let mut f = setup();
    let p = paged_cut(&mut f, true, "presentation");
    let c = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap();
    let p = paged_cut(&mut f, true, "presentation");
    let DiscoveryReplyV1::Page {
        entries,
        continuation: Some(next),
        ..
    } = f
        .owner
        .query_discovery_v1(
            &c.context,
            &scope(),
            &p,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("first page");
    };
    assert_eq!(entries.len(), 1);
    let first = entries[0].reference.clone();
    let after_first = f.owner.discovery_usage_v1().unwrap();
    assert_eq!((after_first.selectors, after_first.issued_lifetime), (2, 3));
    let p = paged_cut(&mut f, true, "presentation");
    assert!(
        f.owner
            .query_discovery_v1(
                &c.context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: Some(&next)
                },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().issued_lifetime,
        after_first.issued_lifetime
    );
    let other = acquire(&mut f, true, true);
    assert!(
        query(
            &mut f,
            &other.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: Some(&next)
            },
            true
        )
        .is_err()
    );
    let p = paged_cut(&mut f, true, "presentation");
    let DiscoveryReplyV1::Page {
        entries,
        continuation,
        ..
    } = f
        .owner
        .query_discovery_v1(
            &c.context,
            &scope(),
            &p,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: Some(&next),
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("second page");
    };
    assert_eq!(entries.len(), 1);
    assert_ne!(entries[0].reference, first);
    assert!(continuation.is_none());
    let issued = f.owner.discovery_usage_v1().unwrap().issued_lifetime;
    let p = paged_cut(&mut f, true, "presentation");
    let DiscoveryReplyV1::Page {
        entries,
        continuation,
        ..
    } = f
        .owner
        .query_discovery_v1(
            &c.context,
            &scope(),
            &p,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("repeat");
    };
    assert_eq!(entries[0].reference, first);
    assert_eq!(continuation, Some(next));
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().issued_lifetime,
        issued
    );
}

#[test]
fn new_entries_cannot_widen_capture_and_changed_permitted_values_stop_it() {
    let mut f = setup();
    let p = paged_cut(&mut f, false, "original");
    let c = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap();
    let p = paged_cut(&mut f, true, "original");
    let DiscoveryReplyV1::Page {
        entries,
        continuation,
        ..
    } = f
        .owner
        .query_discovery_v1(
            &c.context,
            &scope(),
            &p,
            DiscoveryQueryV1::List {
                limit: 4,
                continuation: None,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("page");
    };
    assert_eq!(entries.len(), 1);
    assert!(continuation.is_none());
    let p = paged_cut(&mut f, true, "changed permitted presentation");
    assert!(
        f.owner
            .query_discovery_v1(
                &c.context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 4,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert!(
        f.owner
            .discovery_context_usage_v1(&c.context)
            .unwrap()
            .stopped
    );
}

#[test]
fn query_selector_and_reply_limits_consume_cuts_without_publishing_partial_issuance() {
    for constraint in 0..4 {
        let mut f = support::Fixture::new();
        f.owner
            .configure_retention_budget(4194304, &mut f.recorder)
            .unwrap();
        let mut l = limits();
        match constraint {
            0 => l.max_selectors = 1,
            1 => l.max_issued = 2,
            2 => l.max_selector_bytes = 0,
            _ => l.max_reply_bytes = std::mem::size_of::<DiscoveryReplyV1>(),
        }
        f.owner
            .configure_discovery_v1(&scope(), l, &facts(), &mut f.recorder)
            .unwrap();
        let p = paged_cut(&mut f, true, "presentation");
        let c = f
            .owner
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
            .unwrap();
        let before = f.owner.discovery_usage_v1().unwrap();
        let p = paged_cut(&mut f, true, "presentation");
        assert!(matches!(
            f.owner.query_discovery_v1(
                &c.context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            ),
            Err(DiscoveryErrorV1::Limit)
        ));
        let after = f.owner.discovery_usage_v1().unwrap();
        assert_eq!(
            (after.issued_lifetime, after.selectors, after.preparations),
            (before.issued_lifetime, before.selectors, 0)
        );
        assert!(
            !f.owner
                .discovery_context_usage_v1(&c.context)
                .unwrap()
                .stopped
        );
    }
}

#[test]
fn advice_requires_original_variant_target_and_current_independent_truth() {
    for field in 0..3 {
        for missing in [None, Some(false)] {
            let mut f = setup();
            let c = acquire(&mut f, true, true);
            let e = entry(&mut f, &c.context, true);
            let DiscoveryReplyV1::Descriptor { descriptor: d, .. } = query(
                &mut f,
                &c.context,
                DiscoveryQueryV1::Inspect { entry: &e },
                true,
            )
            .unwrap() else {
                panic!("descriptor");
            };
            let other = acquire(&mut f, true, true);
            assert!(
                query(
                    &mut f,
                    &c.context,
                    DiscoveryQueryV1::AdvisoryEnablement {
                        entry: &e,
                        variant: &d.variants[0].reference,
                        target: other.target.as_ref().unwrap()
                    },
                    true
                )
                .is_err()
            );
            let p = cut(&mut f, true, true);
            let mut g = facts();
            *match field {
                0 => &mut g.advice_attested,
                1 => &mut g.target_current,
                _ => &mut g.target_equality_observable,
            } = missing;
            assert!(
                f.owner
                    .query_discovery_v1(
                        &c.context,
                        &scope(),
                        &p,
                        DiscoveryQueryV1::AdvisoryEnablement {
                            entry: &e,
                            variant: &d.variants[0].reference,
                            target: c.target.as_ref().unwrap()
                        },
                        &g,
                        &mut f.recorder
                    )
                    .is_err()
            );
            assert!(
                !f.owner
                    .discovery_context_usage_v1(&c.context)
                    .unwrap()
                    .stopped
            );
            assert!(
                query(
                    &mut f,
                    &c.context,
                    DiscoveryQueryV1::AdvisoryEnablement {
                        entry: &e,
                        variant: &d.variants[0].reference,
                        target: c.target.as_ref().unwrap()
                    },
                    true
                )
                .is_ok()
            );
        }
    }
}

#[test]
fn exact_native_page_reply_boundary_and_invalid_limits_do_not_publish_selectors() {
    for delta in [-1, 0, 1] {
        let mut f = support::Fixture::new();
        f.owner
            .configure_retention_budget(4194304, &mut f.recorder)
            .unwrap();
        let mut l = limits();
        l.max_reply_bytes = (std::mem::size_of::<DiscoveryReplyV1>()
            + std::mem::size_of::<DiscoveryListEntryV1>())
        .checked_add_signed(delta)
        .unwrap();
        f.owner
            .configure_discovery_v1(&scope(), l, &facts(), &mut f.recorder)
            .unwrap();
        let c = acquire(&mut f, true, true);
        let result = query(
            &mut f,
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            true,
        );
        assert_eq!(result.is_ok(), delta >= 0);
        assert_eq!(
            f.owner.discovery_usage_v1().unwrap().selectors,
            usize::from(delta >= 0)
        );
    }
    for limit in [0, limits().max_page_rows + 1, usize::MAX] {
        let mut f = setup();
        let c = acquire(&mut f, true, true);
        assert!(matches!(
            query(
                &mut f,
                &c.context,
                DiscoveryQueryV1::List {
                    limit,
                    continuation: None
                },
                true
            ),
            Err(DiscoveryErrorV1::InvalidRequest)
        ));
        let u = f.owner.discovery_usage_v1().unwrap();
        // Scalar-shape refusal precedes consumption; a well-shaped request
        // exceeding the current page ceiling still reaches native admission.
        let unconsumed = usize::from(limit == 0 || (limit as u128) > 9_007_199_254_740_991);
        assert_eq!(
            (u.selectors, u.preparations, u.issued_lifetime),
            (0, unconsumed, 2)
        );
    }
}

#[test]
fn target_mismatch_and_unknown_packet_advice_never_become_false_advice() {
    for case in 0..3 {
        let mut f = setup();
        let c = acquire(&mut f, true, true);
        let e = entry(&mut f, &c.context, true);
        let DiscoveryReplyV1::Descriptor { descriptor: d, .. } = query(
            &mut f,
            &c.context,
            DiscoveryQueryV1::Inspect { entry: &e },
            true,
        )
        .unwrap() else {
            panic!("descriptor");
        };
        let p = metadata(true, true, |mut input| {
            let mut g = facts();
            match case {
                0 => {
                    input.target = Some(DiscoveryTargetInputV1 {
                        identity: "target",
                        basis: D::Number(2.0),
                    })
                }
                1 => g.advice_attested = None,
                _ => g.target_binding_attested = None,
            }
            f.owner
                .prepare_discovery_cut_v1(&scope(), input, &g, &mut f.recorder)
        })
        .unwrap();
        assert!(matches!(
            f.owner.query_discovery_v1(
                &c.context,
                &scope(),
                &p,
                DiscoveryQueryV1::AdvisoryEnablement {
                    entry: &e,
                    variant: &d.variants[0].reference,
                    target: c.target.as_ref().unwrap()
                },
                &facts(),
                &mut f.recorder
            ),
            Err(DiscoveryErrorV1::Unavailable)
        ));
        assert!(
            !f.owner
                .discovery_context_usage_v1(&c.context)
                .unwrap()
                .stopped
        );
    }
}

#[test]
fn nonpublication_resource_registration_invalidates_old_discovery_cuts_without_changing_authored_revisions()
 {
    for kind in 0..3 {
        let mut f = setup();
        let c = acquire(&mut f, true, true);
        let old = cut(&mut f, true, true);
        let generation = f.owner.generation();
        let source = f
            .owner
            .read(&f.doc.document, &mut f.recorder)
            .unwrap()
            .version;
        let before = f.owner.discovery_usage_v1().unwrap();
        match kind {
            0 => {
                f.owner
                    .register_provider(
                        vellum_engine::resource::ProviderProfile {
                            synchronization_supported: false,
                        },
                        None,
                        &mut f.recorder,
                    )
                    .unwrap();
            }
            1 => {
                f.owner
                    .register_annotation_encoder(&mut f.recorder)
                    .unwrap();
            }
            _ => {
                f.owner
                    .register_annotation_restorer(&mut f.recorder)
                    .unwrap();
            }
        }
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner
                .read(&f.doc.document, &mut f.recorder)
                .unwrap()
                .version,
            source
        );
        let result = f.owner.query_discovery_v1(
            &c.context,
            &scope(),
            &old,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &facts(),
            &mut f.recorder,
        );
        assert!(matches!(result, Err(DiscoveryErrorV1::Unavailable)));
        let after = f.owner.discovery_usage_v1().unwrap();
        assert_eq!(
            (after.selectors, after.issued_lifetime, after.preparations),
            (
                before.selectors,
                before.issued_lifetime,
                before.preparations
            )
        );
        assert!(
            !f.owner
                .discovery_context_usage_v1(&c.context)
                .unwrap()
                .stopped
        );
        assert!(
            query(
                &mut f,
                &c.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                true
            )
            .is_ok()
        );
        f.owner
            .retire_discovery_cut_v1(&old, &facts(), &mut f.recorder)
            .unwrap();
    }
}

#[test]
fn same_size_administrative_root_changes_refuse_stale_cuts_but_preserve_existing_contexts() {
    for change in 0..9 {
        let mut f = setup();
        let c = acquire(&mut f, true, true);
        let old = cut(&mut f, true, true);
        let generation = f.owner.generation();
        let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
        match change {
            0 => f
                .owner
                .configure_workspaces(
                    WorkspaceLimits {
                        max_issued: 8,
                        max_live: 4,
                        max_memberships: 16,
                        max_work: 1024,
                    },
                    &mut f.recorder,
                )
                .unwrap(),
            1 => {
                f.owner
                    .configure_logical_views(
                        LogicalViewLimits {
                            max_live_views: 4,
                            max_issued_views: 8,
                            max_total_selections: 16,
                            normalization: vellum_engine::selection::SelectionBudget {
                                max_input_selections: 8,
                                max_output_selections: 8,
                                max_work: 1024,
                            },
                            max_mapping_steps: 1024,
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
            }
            2 => {
                f.owner
                    .configure_line_edits(
                        LineEditBudget {
                            max_lines: 16,
                            max_prefix_bytes: 16,
                            max_indent_columns: 16,
                            max_unit_bytes: 4,
                            max_derived_bytes: 8192,
                            max_source_scan_bytes: 4096,
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
            }
            3 => {
                f.owner
                    .configure_unicode_commands(
                        UnicodeCommandBudget {
                            max_steps: 16,
                            max_derived_bytes: 8192,
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
            }
            4 => f
                .owner
                .configure_source_continuity_v1(
                    SourceContinuityLimitsV1 {
                        max_edges: 4,
                        max_derivation_bytes: 8192,
                        max_receipt_bytes: 8192,
                        max_retained_bytes: 65536,
                    },
                    &mut f.recorder,
                )
                .unwrap(),
            5 => f
                .owner
                .configure_save_operations_v1(
                    SaveOperationLimitsV1 {
                        max_issued: 0,
                        max_label_bytes: 64,
                        max_capture_bytes: 0,
                        max_reply_bytes: 1024,
                        max_store_bytes: 65536,
                    },
                    &mut f.recorder,
                )
                .unwrap(),
            6 => f.owner.stop_annotation_save_admission(&mut f.recorder),
            7 => f.owner.stop_annotation_read_admission_v1(&mut f.recorder),
            _ => {
                assert!(
                    f.owner
                        .attach_annotation_read_baselines(&mut f.recorder)
                        .unwrap()
                );
            }
        }
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
        let before = f.owner.discovery_usage_v1().unwrap();
        assert!(matches!(
            f.owner.query_discovery_v1(
                &c.context,
                &scope(),
                &old,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            ),
            Err(DiscoveryErrorV1::Unavailable)
        ));
        let after = f.owner.discovery_usage_v1().unwrap();
        assert_eq!(
            (after.issued_lifetime, after.selectors, after.preparations),
            (
                before.issued_lifetime,
                before.selectors,
                before.preparations
            )
        );
        assert!(
            !after.faulted
                && !f
                    .owner
                    .discovery_context_usage_v1(&c.context)
                    .unwrap()
                    .stopped
        );
        assert!(
            query(
                &mut f,
                &c.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                true
            )
            .is_ok()
        );
    }
}

#[test]
fn attachment_incarnation_not_presence_or_bytes_prevents_discovery_cut_aba() {
    let mut f = setup();
    f.owner
        .attach_annotation_read_baselines(&mut f.recorder)
        .unwrap();
    let c = acquire(&mut f, true, true);
    let old = cut(&mut f, true, true);
    let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    let generation = f.owner.generation();
    assert!(
        f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert!(
        f.owner
            .attach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
    assert!(matches!(
        f.owner.query_discovery_v1(
            &c.context,
            &scope(),
            &old,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            &facts(),
            &mut f.recorder
        ),
        Err(DiscoveryErrorV1::Unavailable)
    ));
    let current = cut(&mut f, true, true);
    assert!(
        !f.owner
            .attach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert!(
        f.owner
            .query_discovery_v1(
                &c.context,
                &scope(),
                &current,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
}

#[test]
fn absent_attachment_cannot_aba_back_to_an_old_discovery_root() {
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    let old = cut(&mut f, true, true);
    let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    let generation = f.owner.generation();
    assert!(
        f.owner
            .attach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert!(
        f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
    assert!(matches!(
        f.owner.query_discovery_v1(
            &c.context,
            &scope(),
            &old,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            &facts(),
            &mut f.recorder
        ),
        Err(DiscoveryErrorV1::Unavailable)
    ));
    assert!(
        query(
            &mut f,
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            true
        )
        .is_ok()
    );
}

#[test]
fn stale_registration_cut_cannot_acquire_a_new_context_or_spend_issuance() {
    let mut f = setup();
    let old = cut(&mut f, true, true);
    f.owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let before = f.owner.discovery_usage_v1().unwrap();
    assert_eq!(
        f.owner
            .acquire_discovery_v1(&scope(), &old, &facts(), &mut f.recorder),
        Err(DiscoveryErrorV1::Unavailable)
    );
    let after = f.owner.discovery_usage_v1().unwrap();
    assert_eq!(
        (after.contexts, after.issued_lifetime, after.preparations),
        (before.contexts, before.issued_lifetime, before.preparations)
    );
    acquire(&mut f, true, true);
    f.owner
        .retire_discovery_cut_v1(&old, &facts(), &mut f.recorder)
        .unwrap();
}

#[test]
fn private_save_admission_preserves_cut_but_native_dispatch_serial_changes_the_root() {
    use vellum_engine::resource::{Acknowledgment, Protection, SaveRequirements};
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    let before_queue = cut(&mut f, true, true);
    let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    let generation = f.owner.generation();
    let pair = f.pair().unwrap();
    let task = f
        .owner
        .queue_pinned_save_with_pair(
            &f.doc,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut f.recorder,
        )
        .unwrap();
    assert!(
        f.owner
            .query_discovery_v1(
                &c.context,
                &scope(),
                &before_queue,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
    let before_dispatch = cut(&mut f, true, true);
    let pair = f.pair().unwrap();
    f.owner
        .dispatch_save_with_pair(
            &task,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
    assert!(matches!(
        f.owner.query_discovery_v1(
            &c.context,
            &scope(),
            &before_dispatch,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            &facts(),
            &mut f.recorder
        ),
        Err(DiscoveryErrorV1::Unavailable)
    ));
    assert!(
        query(
            &mut f,
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            true
        )
        .is_ok()
    );
}

#[test]
fn idempotent_native_root_profile_and_fence_calls_do_not_stale_a_prepared_cut() {
    for kind in 0..5 {
        let mut f = setup();
        let c = acquire(&mut f, true, true);
        let apply = |f: &mut support::Fixture| match kind {
            0 => {
                f.owner
                    .configure_line_edits(
                        LineEditBudget {
                            max_lines: 16,
                            max_prefix_bytes: 16,
                            max_indent_columns: 16,
                            max_unit_bytes: 4,
                            max_derived_bytes: 8192,
                            max_source_scan_bytes: 4096,
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
            }
            1 => {
                f.owner
                    .configure_unicode_commands(
                        UnicodeCommandBudget {
                            max_steps: 16,
                            max_derived_bytes: 8192,
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
            }
            2 => f
                .owner
                .configure_source_continuity_v1(
                    SourceContinuityLimitsV1 {
                        max_edges: 4,
                        max_derivation_bytes: 8192,
                        max_receipt_bytes: 8192,
                        max_retained_bytes: 65536,
                    },
                    &mut f.recorder,
                )
                .unwrap(),
            3 => f
                .owner
                .configure_save_operations_v1(
                    SaveOperationLimitsV1 {
                        max_issued: 0,
                        max_label_bytes: 64,
                        max_capture_bytes: 0,
                        max_reply_bytes: 1024,
                        max_store_bytes: 65536,
                    },
                    &mut f.recorder,
                )
                .unwrap(),
            _ => {
                f.owner.stop_annotation_save_admission(&mut f.recorder);
                f.owner.stop_annotation_read_admission_v1(&mut f.recorder);
            }
        };
        apply(&mut f);
        let old = cut(&mut f, true, true);
        let generation = f.owner.generation();
        apply(&mut f);
        assert_eq!(f.owner.generation(), generation);
        assert!(
            f.owner
                .query_discovery_v1(
                    &c.context,
                    &scope(),
                    &old,
                    DiscoveryQueryV1::List {
                        limit: 1,
                        continuation: None
                    },
                    &facts(),
                    &mut f.recorder
                )
                .is_ok()
        );
    }
}
