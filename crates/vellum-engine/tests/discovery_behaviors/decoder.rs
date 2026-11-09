use super::*;
use DiscoveryEnvelopeValueV1 as V;
fn decoder_limits() -> DiscoveryDecoderLimitsV1 {
    DiscoveryDecoderLimitsV1 {
        input: CaptureLimitsV1 {
            max_bytes: 8192,
            max_nodes: 256,
            max_depth: 12,
        },
        max_held_bytes: 262144,
    }
}
struct Input<'a, 'h> {
    route: &'a str,
    selector: D<'a>,
    version: D<'a>,
    form: DiscoveryOperationV1,
    variant: &'a str,
    operation: &'a str,
    contract: &'a str,
    profile: D<'a>,
    context: V<'a, 'h>,
    entry: V<'a, 'h>,
    variant_ref: V<'a, 'h>,
    target: V<'a, 'h>,
    limit: D<'a>,
    continuation: V<'a, 'h>,
    extra: bool,
    duplicate: bool,
    header_extra: bool,
    outer_extra: bool,
    omit: bool,
    reverse: bool,
}
impl<'a, 'h> Input<'a, 'h> {
    fn new(route: &'a str, context: &'h DiscoveryContextIdV1, query: DiscoveryQueryV1<'h>) -> Self {
        let form = query.operation();
        let declaration = form.declaration();
        let mut input = Self {
            route,
            selector: if route == "typed" {
                D::Number(declaration.family as f64)
            } else {
                D::Text(declaration.name)
            },
            version: D::Number(declaration.version as f64),
            form,
            variant: form.name(),
            operation: form.name(),
            contract: declaration.contract,
            profile: D::Text(OWNED_CONTRACT_DISCOVERY_PROFILE_V1),
            context: V::Context(context),
            entry: V::Data(D::Null),
            variant_ref: V::Data(D::Null),
            target: V::Data(D::Null),
            limit: D::Number(1.0),
            continuation: V::Data(D::Null),
            extra: false,
            duplicate: false,
            header_extra: false,
            outer_extra: false,
            omit: false,
            reverse: false,
        };
        match query {
            DiscoveryQueryV1::List {
                limit,
                continuation,
            } => {
                input.limit = D::Number(limit as f64);
                if let Some(v) = continuation {
                    input.continuation = V::Cursor(v);
                }
            }
            DiscoveryQueryV1::Inspect { entry } => input.entry = V::Entry(entry),
            DiscoveryQueryV1::AdvisoryEnablement {
                entry,
                variant,
                target,
            } => {
                input.entry = V::Entry(entry);
                input.variant_ref = V::Variant(variant);
                input.target = V::Target(target);
            }
        }
        input
    }
}
fn with_input<'a, 'h, T>(i: &Input<'a, 'h>, run: impl for<'v> FnOnce(&'v V<'v, 'h>) -> T) -> T {
    let mut args = [
        ("contract", V::Data(D::Text(i.contract))),
        ("operation", V::Data(D::Text(i.operation))),
        ("context", i.context),
        ("profile", V::Data(i.profile)),
        ("entry", i.entry),
        ("variant", i.variant_ref),
        ("target", i.target),
        ("extra", V::Data(D::Null)),
    ];
    let mut n = match i.form {
        DiscoveryOperationV1::List => {
            args[4] = ("limit", V::Data(i.limit));
            args[5] = ("continuation", i.continuation);
            6
        }
        DiscoveryOperationV1::Inspect => 5,
        DiscoveryOperationV1::AdvisoryEnablement => 7,
    };
    if i.omit {
        n -= 1;
    }
    if i.extra {
        args[n] = ("extra", V::Data(D::Null));
        n += 1;
    }
    if i.duplicate {
        args[n - 1].0 = "contract";
    }
    if i.reverse {
        args[..n].reverse();
    }
    let header = [
        (
            if i.route == "dynamic" {
                "name"
            } else {
                "family"
            },
            V::Data(i.selector),
        ),
        ("variant", V::Data(D::Text(i.variant))),
        ("version", V::Data(i.version)),
        ("arguments", V::Object(&args[..n])),
        ("extra", V::Data(D::Null)),
    ];
    let outer = [
        ("route", V::Data(D::Text(i.route))),
        (
            "request",
            V::Object(&header[..if i.header_extra { 5 } else { 4 }]),
        ),
        ("extra", V::Data(D::Null)),
    ];
    run(&V::Object(&outer[..if i.outer_extra { 3 } else { 2 }]))
}
fn enable(f: &mut support::Fixture, c: &DiscoveryContextIdV1) {
    f.owner
        .enable_discovery_decoder_v1(c, &scope(), decoder_limits(), &facts(), &mut f.recorder)
        .unwrap();
}
fn decoded(
    f: &mut support::Fixture,
    c: &DiscoveryContextIdV1,
    query: DiscoveryQueryV1<'_>,
    route: &str,
) -> Result<DiscoveryReplyV1, DiscoveryErrorV1> {
    let p = cut(f, true, true);
    with_input(&Input::new(route, c, query), |e| {
        f.owner
            .query_decoded_discovery_v1(c, &scope(), &p, e, &facts(), &mut f.recorder)
    })
}
#[test]
fn declarations_match_both_native_routes_and_preserve_every_original_role_and_full_epoch() {
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
    let forms = native_discovery_input_forms_v1();
    assert_eq!(
        forms.map(|v| v.operation.name()),
        ["list", "inspect", "advisory-enablement"]
    );
    for (q, form) in [
        DiscoveryQueryV1::List {
            limit: 1,
            continuation: None,
        },
        DiscoveryQueryV1::Inspect { entry: &e },
        DiscoveryQueryV1::AdvisoryEnablement {
            entry: &e,
            variant: &d.variants[0].reference,
            target: c.target.as_ref().unwrap(),
        },
    ]
    .into_iter()
    .zip(forms)
    {
        assert_eq!(
            (form.family, form.name, form.version, form.contract),
            (0, "discover-contracts", 1, "contract-discovery/1")
        );
        assert_eq!(q.operation(), form.operation);
        for epoch in [0, 9_007_199_254_740_993, u64::MAX] {
            let mut s = scope();
            s.policy_epoch = epoch;
            let typed = with_input(&Input::new("typed", &c.context, q), |e| {
                normalize_discovery_arguments_v1(
                    e,
                    &s,
                    decoder_limits().input,
                    8192,
                    &mut f.recorder,
                )
            })
            .unwrap();
            let mut dynamic = Input::new("dynamic", &c.context, q);
            dynamic.reverse = true;
            let dynamic = with_input(&dynamic, |e| {
                normalize_discovery_arguments_v1(
                    e,
                    &s,
                    decoder_limits().input,
                    8192,
                    &mut f.recorder,
                )
            })
            .unwrap();
            assert_eq!(typed, dynamic);
            assert_eq!(typed.context(), &c.context);
            assert_eq!(typed.query(), q);
            assert_eq!(typed.scope(), &s);
            assert_eq!(typed.scope().policy_epoch, epoch);
            assert!(typed.capture_usage().iter().all(|u| u.peak_bytes <= 8192));
            assert!(!format!("{typed:?}").contains("recipient"));
        }
    }
}
#[test]
fn decoded_list_inspect_and_false_advice_use_real_native_guards_without_new_admissions() {
    for capacity in [None, Some(0), Some(256)] {
        let mut f = setup();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = acquire(&mut f, true, true);
        enable(&mut f, &c.context);
        let funding = f.owner.retention_usage();
        let generation = f.owner.generation();
        let mut first = None;
        for route in ["typed", "dynamic"] {
            let DiscoveryReplyV1::Page {
                context,
                entries,
                continuation,
                ..
            } = decoded(
                &mut f,
                &c.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                route,
            )
            .unwrap()
            else {
                panic!("page");
            };
            assert_eq!(context, c.context);
            assert!(continuation.is_none());
            let e = entries[0].reference.clone();
            if let Some(first) = &first {
                assert_eq!(first, &e);
            } else {
                first = Some(e.clone());
            }
            let DiscoveryReplyV1::Descriptor { descriptor: d, .. } = decoded(
                &mut f,
                &c.context,
                DiscoveryQueryV1::Inspect { entry: &e },
                route,
            )
            .unwrap() else {
                panic!("descriptor");
            };
            assert!(d.variants[0].schema.is_some() && d.presentation.is_some());
            let v = &d.variants[0].reference;
            let target = c.target.as_ref().unwrap();
            let DiscoveryReplyV1::Advice {
                entry,
                variant,
                target: t,
                enabled,
                ..
            } = decoded(
                &mut f,
                &c.context,
                DiscoveryQueryV1::AdvisoryEnablement {
                    entry: &e,
                    variant: v,
                    target,
                },
                route,
            )
            .unwrap()
            else {
                panic!("advice");
            };
            assert_eq!(
                (entry, variant, t, enabled),
                (e, v.clone(), target.clone(), false)
            );
        }
        assert_eq!(f.owner.retention_usage(), funding);
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(f.owner.discovery_usage_v1().unwrap().contexts, 1);
        assert!(!f.owner.discovery_decoder_usage_v1(&c.context).unwrap().busy);
        let mut events = String::new();
        while let Some(e) = f.recorder.pop_front() {
            events.push_str(&format!("{e:?}"));
        }
        for label in ["recipient", "literal untrusted metadata", "example/1"] {
            assert!(!events.contains(label));
        }
        if capacity == Some(256) {
            for marker in [
                "DiscoveryArgumentNormalization",
                "DiscoveryDecoderQuery",
                "DiscoveryResultValidation",
            ] {
                assert!(events.contains(marker));
            }
        }
    }
}
#[test]
fn decoder_is_explicit_funded_once_and_refunds_only_actual_stopped_context_retirement() {
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    let p = cut(&mut f, true, true);
    let before = f.owner.retention_usage();
    with_input(
        &Input::new(
            "typed",
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
        ),
        |e| {
            assert!(matches!(
                f.owner.query_decoded_discovery_v1(
                    &c.context,
                    &scope(),
                    &p,
                    e,
                    &facts(),
                    &mut f.recorder
                ),
                Err(DiscoveryErrorV1::Unavailable)
            ));
        },
    );
    assert_eq!(f.owner.retention_usage(), before);
    enable(&mut f, &c.context);
    let charge = decoder_limits().reserved_bytes().unwrap();
    assert_eq!(f.owner.retention_usage().reserved, before.reserved + charge);
    enable(&mut f, &c.context);
    assert_eq!(f.owner.retention_usage().reserved, before.reserved + charge);
    with_input(
        &Input::new(
            "typed",
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
        ),
        |e| {
            f.owner.query_decoded_discovery_v1(
                &c.context,
                &scope(),
                &p,
                e,
                &facts(),
                &mut f.recorder,
            )
        },
    )
    .unwrap();
    f.owner
        .stop_discovery_context_v1(&c.context, &scope(), &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, before.reserved + charge);
    let mut blocked = facts();
    blocked.delivery_drained = None;
    assert!(
        f.owner
            .retire_discovery_context_v1(&c.context, &blocked, &mut f.recorder)
            .is_err()
    );
    f.owner
        .retire_discovery_context_v1(&c.context, &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage(), before);
    assert!(
        f.owner
            .retire_discovery_context_v1(&c.context, &facts(), &mut f.recorder)
            .is_err()
    );
    f.owner
        .stop_discovery_admission_v1(&facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_registry_v1(&facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
}
#[test]
fn malformed_envelopes_and_wrong_role_kinds_do_not_consume_cuts_or_poison_contexts() {
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    enable(&mut f, &c.context);
    let funding = f.owner.retention_usage();
    for route in ["typed", "dynamic"] {
        for change in 0..21 {
            let mut i = Input::new(
                route,
                &c.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
            );
            match change {
                0 => i.route = "remote",
                1 => i.selector = D::Text("not-a-handler"),
                2 => i.version = D::Number(2.0),
                3 => i.variant = "unknown",
                4 => i.operation = "inspect",
                5 => i.contract = "unknown/1",
                6 => i.profile = D::Text(""),
                7 => i.context = V::Data(D::Object(&[])),
                8 => i.continuation = V::Data(D::Object(&[])),
                9 => i.omit = true,
                10 => i.extra = true,
                11 => i.duplicate = true,
                12 => i.header_extra = true,
                13 => i.outer_extra = true,
                14 => i.limit = D::Number(0.0),
                15 => i.limit = D::Number(-1.0),
                16 => i.limit = D::Number(0.5),
                17 => i.limit = D::Number(f64::NAN),
                18 => i.limit = D::Number(f64::INFINITY),
                19 => i.limit = D::Number(9_007_199_254_740_992.0),
                _ => i.context = V::Data(D::Null),
            }
            let p = cut(&mut f, true, true);
            let before = f.owner.discovery_usage_v1().unwrap();
            let mut g = facts();
            g.authorized = Some(false);
            assert!(matches!(
                with_input(&i, |e| f.owner.query_decoded_discovery_v1(
                    &c.context,
                    &scope(),
                    &p,
                    e,
                    &g,
                    &mut f.recorder
                )),
                Err(DiscoveryErrorV1::InvalidRequest)
            ));
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
                !after.faulted
                    && !f
                        .owner
                        .discovery_context_usage_v1(&c.context)
                        .unwrap()
                        .stopped
            );
            assert!(!f.owner.discovery_decoder_usage_v1(&c.context).unwrap().busy);
            f.owner
                .retire_discovery_cut_v1(&p, &facts(), &mut f.recorder)
                .unwrap();
        }
    }
    assert_eq!(f.owner.retention_usage(), funding);
}
#[test]
fn native_argument_byte_node_depth_and_caller_ceilings_have_exact_boundaries() {
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    for route in ["typed", "dynamic"] {
        let input = Input::new(
            route,
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
        );
        let n = with_input(&input, |e| {
            normalize_discovery_arguments_v1(
                e,
                &scope(),
                decoder_limits().input,
                8192,
                &mut f.recorder,
            )
        })
        .unwrap();
        let bytes = n
            .capture_usage()
            .iter()
            .map(|u| u.peak_bytes)
            .max()
            .unwrap()
            .max(n.owned_bytes())
            .max(DiscoveryDecoderLimitsV1::minimum_input_bytes());
        let nodes = n.capture_usage().iter().map(|u| u.nodes).max().unwrap();
        let depth = n.capture_usage().iter().map(|u| u.depth).max().unwrap();
        for field in 0..4 {
            for delta in [-1, 0, 1] {
                let mut l = decoder_limits().input;
                let mut maximum = 8192;
                match field {
                    0 => l.max_bytes = bytes.checked_add_signed(delta).unwrap(),
                    1 => l.max_nodes = nodes.checked_add_signed(delta).unwrap(),
                    2 => l.max_depth = depth.checked_add_signed(delta).unwrap(),
                    _ => maximum = bytes.checked_add_signed(delta).unwrap(),
                }
                let result = with_input(&input, |e| {
                    normalize_discovery_arguments_v1(e, &scope(), l, maximum, &mut f.recorder)
                });
                assert_eq!(result.is_ok(), delta >= 0, "field {field}, delta {delta}");
            }
        }
    }
}
#[test]
fn decoder_declared_and_actual_parent_bounds_refuse_without_installing_or_spending() {
    for field in 0..2 {
        for delta in [-1, 0, 1] {
            let mut l = decoder_limits();
            let charge = l.reserved_bytes().unwrap();
            if field == 0 {
                l.max_held_bytes = charge.checked_add_signed(delta).unwrap();
            }
            let maximum = if field == 1 {
                (limits().max_held_bytes + charge)
                    .checked_add_signed(delta)
                    .unwrap()
            } else {
                4194304
            };
            let mut f = support::Fixture::new();
            f.owner
                .configure_retention_budget(maximum, &mut f.recorder)
                .unwrap();
            f.owner
                .configure_discovery_v1(&scope(), limits(), &facts(), &mut f.recorder)
                .unwrap();
            let c = acquire(&mut f, true, true);
            let before = f.owner.retention_usage();
            let result = f.owner.enable_discovery_decoder_v1(
                &c.context,
                &scope(),
                l,
                &facts(),
                &mut f.recorder,
            );
            assert_eq!(result.is_ok(), delta >= 0);
            if delta < 0 {
                assert_eq!(result, Err(DiscoveryErrorV1::Limit));
                assert_eq!(f.owner.retention_usage(), before);
                assert!(f.owner.discovery_decoder_usage_v1(&c.context).is_err());
            } else {
                assert_eq!(f.owner.retention_usage().reserved, before.reserved + charge);
            }
        }
    }
}
#[test]
fn foreign_context_profile_or_scope_cannot_apply_loss_to_the_bound_decoder() {
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    let other = acquire(&mut f, true, true);
    enable(&mut f, &c.context);
    for change in 0..3 {
        let p = cut(&mut f, true, true);
        let mut i = Input::new(
            "dynamic",
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
        );
        let mut s = scope();
        let mut g = facts();
        g.authorized = Some(false);
        match change {
            0 => i.context = V::Context(&other.context),
            1 => i.profile = D::Text("owned-contract-discovery/13"),
            _ => s.policy_epoch += 1,
        }
        assert!(matches!(
            with_input(&i, |e| f.owner.query_decoded_discovery_v1(
                &c.context,
                &s,
                &p,
                e,
                &g,
                &mut f.recorder
            )),
            Err(DiscoveryErrorV1::Unavailable)
        ));
        assert!(
            !f.owner
                .discovery_context_usage_v1(&c.context)
                .unwrap()
                .stopped
        );
        f.owner
            .retire_discovery_cut_v1(&p, &facts(), &mut f.recorder)
            .unwrap();
    }
    assert!(
        decoded(
            &mut f,
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            "typed"
        )
        .is_ok()
    );
}
#[test]
fn current_fact_copy_preserves_unknown_vs_explicit_loss_and_keeps_original_workspace_credit() {
    for flag in 0..8 {
        for value in [None, Some(false)] {
            let mut f = setup();
            let c = acquire(&mut f, true, true);
            enable(&mut f, &c.context);
            let funding = f.owner.retention_usage();
            let p = cut(&mut f, true, true);
            let mut g = facts();
            *match flag {
                0 => &mut g.authorized,
                1 => &mut g.scope,
                2 => &mut g.continuous,
                3 => &mut g.live,
                4 => &mut g.not_stopped,
                5 => &mut g.discovery_allowed,
                6 => &mut g.interpretation_current,
                _ => &mut g.retained,
            } = value;
            assert!(matches!(
                with_input(
                    &Input::new(
                        "typed",
                        &c.context,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None
                        }
                    ),
                    |e| f.owner.query_decoded_discovery_v1(
                        &c.context,
                        &scope(),
                        &p,
                        e,
                        &g,
                        &mut f.recorder
                    )
                ),
                Err(DiscoveryErrorV1::Unavailable)
            ));
            assert_eq!(
                f.owner
                    .discovery_context_usage_v1(&c.context)
                    .unwrap()
                    .stopped,
                value == Some(false)
            );
            assert_eq!(f.owner.retention_usage(), funding);
            assert!(!f.owner.discovery_decoder_usage_v1(&c.context).unwrap().busy);
            if value.is_none() {
                with_input(
                    &Input::new(
                        "typed",
                        &c.context,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                    ),
                    |e| {
                        f.owner.query_decoded_discovery_v1(
                            &c.context,
                            &scope(),
                            &p,
                            e,
                            &facts(),
                            &mut f.recorder,
                        )
                    },
                )
                .unwrap();
            }
        }
    }
}
#[test]
fn stopped_new_admissions_do_not_end_existing_prepaid_decoder_queries() {
    let mut f = setup();
    let c = acquire(&mut f, true, true);
    let other = acquire(&mut f, true, true);
    enable(&mut f, &c.context);
    f.owner
        .stop_discovery_admission_v1(&facts(), &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .enable_discovery_decoder_v1(
                &other.context,
                &scope(),
                decoder_limits(),
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert!(
        decoded(
            &mut f,
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            "dynamic"
        )
        .is_ok()
    );
}
#[test]
fn decoded_native_scope_epoch_above_json_integer_range_remains_exact_at_execution() {
    let mut f = setup();
    let mut s = scope();
    s.policy_epoch = u64::MAX;
    let p = metadata(true, true, |i| {
        f.owner
            .prepare_discovery_cut_v1(&s, i, &facts(), &mut f.recorder)
    })
    .unwrap();
    let c = f
        .owner
        .acquire_discovery_v1(&s, &p, &facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .enable_discovery_decoder_v1(&c.context, &s, decoder_limits(), &facts(), &mut f.recorder)
        .unwrap();
    for route in ["typed", "dynamic"] {
        let p = metadata(true, true, |i| {
            f.owner
                .prepare_discovery_cut_v1(&s, i, &facts(), &mut f.recorder)
        })
        .unwrap();
        let reply = with_input(
            &Input::new(
                route,
                &c.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
            ),
            |e| {
                f.owner
                    .query_decoded_discovery_v1(&c.context, &s, &p, e, &facts(), &mut f.recorder)
            },
        )
        .unwrap();
        let DiscoveryReplyV1::Page {
            context, entries, ..
        } = reply
        else {
            panic!("page");
        };
        assert_eq!(context, c.context);
        assert_eq!(entries.len(), 1);
    }
}
#[test]
fn decoded_cursor_is_original_context_and_limit_bound_not_copied_metadata() {
    let mut f = setup();
    let p = paged_cut(&mut f, true, "presentation");
    let c = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap();
    enable(&mut f, &c.context);
    let p = paged_cut(&mut f, true, "presentation");
    let DiscoveryReplyV1::Page {
        continuation: Some(next),
        ..
    } = with_input(
        &Input::new(
            "typed",
            &c.context,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
        ),
        |e| {
            f.owner.query_decoded_discovery_v1(
                &c.context,
                &scope(),
                &p,
                e,
                &facts(),
                &mut f.recorder,
            )
        },
    )
    .unwrap()
    else {
        panic!("first page");
    };
    for route in ["typed", "dynamic"] {
        let q = DiscoveryQueryV1::List {
            limit: 1,
            continuation: Some(&next),
        };
        let n = with_input(&Input::new(route, &c.context, q), |e| {
            normalize_discovery_arguments_v1(
                e,
                &scope(),
                decoder_limits().input,
                8192,
                &mut f.recorder,
            )
        })
        .unwrap();
        assert_eq!(n.query(), q);
        let p = paged_cut(&mut f, true, "presentation");
        let DiscoveryReplyV1::Page {
            entries,
            continuation,
            ..
        } = with_input(&Input::new(route, &c.context, q), |e| {
            f.owner.query_decoded_discovery_v1(
                &c.context,
                &scope(),
                &p,
                e,
                &facts(),
                &mut f.recorder,
            )
        })
        .unwrap()
        else {
            panic!("second page");
        };
        assert_eq!(entries.len(), 1);
        assert!(continuation.is_none());
    }
    let p = paged_cut(&mut f, true, "presentation");
    let before = f.owner.discovery_usage_v1().unwrap();
    assert!(matches!(
        with_input(
            &Input::new(
                "dynamic",
                &c.context,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: Some(&next)
                }
            ),
            |e| f.owner.query_decoded_discovery_v1(
                &c.context,
                &scope(),
                &p,
                e,
                &facts(),
                &mut f.recorder
            )
        ),
        Err(DiscoveryErrorV1::Unavailable)
    ));
    let after = f.owner.discovery_usage_v1().unwrap();
    assert_eq!(after.issued_lifetime, before.issued_lifetime);
    // A mismatched original cursor binding now refuses before cut consumption.
    assert_eq!(after.preparations, before.preparations);
    assert!(
        !f.owner
            .discovery_context_usage_v1(&c.context)
            .unwrap()
            .stopped
    );
}

#[test]
fn decoded_foreign_discovery_roles_cannot_apply_loss_or_spend_a_current_cut() {
    use super::recognition::{invalid_queries, loss_facts, role_cut, roles};
    let mut f = setup();
    let a = roles(&mut f);
    let b = roles(&mut f);
    enable(&mut f, &a.context);
    let reserved = f.owner.retention_usage().reserved;
    for route in ["typed", "dynamic"] {
        for field in 0..8 {
            for q in invalid_queries(&a, &b) {
                let p = role_cut(&mut f);
                let before = f.owner.discovery_usage_v1().unwrap();
                assert!(
                    with_input(&Input::new(route, &a.context, q), |e| f
                        .owner
                        .query_decoded_discovery_v1(
                            &a.context,
                            &scope(),
                            &p,
                            e,
                            &loss_facts(field),
                            &mut f.recorder
                        ))
                    .is_err()
                );
                assert!(
                    !f.owner
                        .discovery_context_usage_v1(&a.context)
                        .unwrap()
                        .stopped
                );
                assert!(
                    !f.owner
                        .discovery_context_usage_v1(&b.context)
                        .unwrap()
                        .stopped
                );
                assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
                assert_eq!(f.owner.retention_usage().reserved, reserved);
                f.owner
                    .retire_discovery_cut_v1(&p, &facts(), &mut f.recorder)
                    .unwrap();
            }
        }
        let p = role_cut(&mut f);
        assert!(
            with_input(
                &Input::new(
                    route,
                    &a.context,
                    DiscoveryQueryV1::Inspect { entry: &a.entry }
                ),
                |e| f.owner.query_decoded_discovery_v1(
                    &a.context,
                    &scope(),
                    &p,
                    e,
                    &facts(),
                    &mut f.recorder
                )
            )
            .is_ok()
        );
    }
}

#[test]
fn decoded_recognized_discovery_loss_is_absorbing_for_both_routes_and_all_forms() {
    use super::recognition::{loss_facts, role_cut, roles};
    for route in ["typed", "dynamic"] {
        for form in 0..3 {
            let mut f = setup();
            let a = roles(&mut f);
            enable(&mut f, &a.context);
            let q = match form {
                0 => DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: Some(&a.cursor),
                },
                1 => DiscoveryQueryV1::Inspect { entry: &a.entry },
                _ => DiscoveryQueryV1::AdvisoryEnablement {
                    entry: &a.entry,
                    variant: &a.variant,
                    target: &a.target,
                },
            };
            let p = role_cut(&mut f);
            assert!(
                with_input(&Input::new(route, &a.context, q), |e| f
                    .owner
                    .query_decoded_discovery_v1(
                        &a.context,
                        &scope(),
                        &p,
                        e,
                        &loss_facts(0),
                        &mut f.recorder
                    ))
                .is_err()
            );
            assert!(
                f.owner
                    .discovery_context_usage_v1(&a.context)
                    .unwrap()
                    .stopped
            );
        }
    }
}
