use super::*;
fn visibility() -> NativeBuiltinEntryVisibilityV1<3> {
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
    generic: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if generic {
        let operations = [
            (0, "literal-matches"),
            (1, "next-match"),
            (2, "previous-match"),
        ];
        let roles: Vec<_> = LiteralSearchOperationV1::LiteralMatches
            .argument_declaration_v1()
            .variant
            .fields
            .iter()
            .map(|(name, role)| (*name, D::Text(role)))
            .collect();
        let inputs: [[_; 3]; 3] = std::array::from_fn(|i| {
            [
                ("contract", D::Text("literal-search/1")),
                ("variant", D::Text(operations[i].1)),
                ("fields", D::Object(&roles)),
            ]
        });
        let variants = operations.map(|(order, id)| DiscoveryVariantInputV1 {
            id,
            input: D::Object(&inputs[order]),
            requirements: D::Object(&[]),
            outcomes: D::Null,
            limits: D::Null,
            cancellation: D::Null,
            order: Some(order),
            enabled: None,
        });
        let entries = [DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority: "authority",
                id: "F05-literal-search-v1",
                incarnation: 1,
            },
            contract: "literal-search/1",
            version: 1,
            kind: DiscoveryExecutionKindV1::Query,
            variants: &variants,
            presentation: D::Object(&[]),
            order: Some(0),
            schema_visible: true,
            presentation_visible: false,
            advice_visible: false,
        }];
        return o
            .prepare_discovery_cut_v1(
                s,
                DiscoveryCutInputV1 {
                    entries: &entries,
                    target: None,
                    advice_present: false,
                },
                &discovery_support::facts(),
                r,
            )
            .unwrap();
    }
    o.prepare_native_literal_search_discovery_cut_v1(
        s,
        &visibility(),
        &discovery_support::facts(),
        r,
    )
    .unwrap()
}
#[test]
fn installed_literal_search_both_routes_reach_original_owner_and_current_result_release() {
    for (capacity, timed) in [(0, false), (256, false), (256, true), (1, true)] {
        for (typed, generic) in [(false, false), (true, false), (false, true), (true, true)] {
            for (index, operation) in ["literal-matches", "next-match", "previous-match"]
                .into_iter()
                .enumerate()
            {
                let (mut o, c, mut r, d, s) = binding::setup();
                let before = o.retention_usage().reserved;
                let scope = DiscoveryScopeV1 {
                    context: SettingsExecutionContextV1 {
                        profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                        ..s.context.clone()
                    },
                    policy_epoch: 1,
                };
                let mut l = discovery_support::limits();
                l.max_contexts = 1;
                l.max_preparations = 1;
                l.max_entries = 1;
                l.max_variants = 3;
                l.max_selectors = 8;
                l.max_issued = 32;
                l.max_page_rows = 1;
                l.max_context_bytes = 256;
                l.max_capture_bytes = 65536;
                l.max_packet_bytes = 65536;
                l.max_reply_bytes = 65536;
                l.max_row_bytes = 32768;
                l.max_root_bytes = 65536;
                l.metadata.max_bytes = 8192;
                l.metadata.max_nodes = 512;
                l.metadata.max_depth = 12;
                l.max_held_bytes = l.declared_bytes().unwrap();
                o.configure_discovery_v1(&scope, l, &discovery_support::facts(), &mut r)
                    .unwrap();
                let p = prepare(&mut o, &scope, generic, &mut r);
                let context = o
                    .acquire_discovery_v1(&scope, &p, &discovery_support::facts(), &mut r)
                    .unwrap()
                    .context;
                let p = prepare(&mut o, &scope, generic, &mut r);
                let DiscoveryReplyV1::Page { entries, .. } = o
                    .query_discovery_v1(
                        &context,
                        &scope,
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                        &discovery_support::facts(),
                        &mut r,
                    )
                    .unwrap()
                else {
                    panic!("page")
                };
                let entry = &entries[0].reference;
                let p = prepare(&mut o, &scope, generic, &mut r);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &context,
                        &scope,
                        &p,
                        DiscoveryQueryV1::Inspect { entry },
                        &discovery_support::facts(),
                        &mut r,
                    )
                    .unwrap()
                else {
                    panic!("descriptor")
                };
                assert_eq!(descriptor.variants.len(), 3);
                let reference = LiteralSearchReferenceV1 {
                    authority: "authority".into(),
                    document: "PRIVATE-source".into(),
                    incarnation: 7,
                    revision: d.revision,
                };
                let binding = o
                    .bind_logical_search_source_v1(
                        TrustedLogicalSearchBindingV1 {
                            source: SearchReadContextV1::Source(&c),
                            scope: &s,
                            expected: &d,
                            reference: &reference,
                            generation: 0,
                            binding_allowed: Some(true),
                            correspondence_current: Some(true),
                            max_label_bytes: 128,
                            max_binding_bytes: 4096,
                        },
                        &mut r,
                    )
                    .unwrap();
                let reference_data = [
                    ("authority", D::Text("authority")),
                    ("document", D::Text("PRIVATE-source")),
                    ("incarnation", D::Number(7.0)),
                    ("revision", D::Number(d.revision as f64)),
                ];
                let range = [("lo", D::Number(0.0)), ("hi", D::Number(6.0))];
                let scopes = [D::Object(&range)];
                let nav = [
                    ("start", D::Number(if index == 2 { 6.0 } else { 0.0 })),
                    ("current", D::Null),
                    ("wrap", D::Boolean(false)),
                ];
                let args = fields(
                    SearchReadContextV1::Source(&c),
                    operation,
                    D::Object(&reference_data),
                    "a",
                    LITERAL_EXACT_MATCHING_PROFILE_V1,
                    D::Array(&scopes),
                    if index == 0 { D::Null } else { D::Object(&nav) },
                );
                let request = [
                    (
                        if typed { "family" } else { "name" },
                        V::Data(if typed {
                            D::Number(4.0)
                        } else {
                            D::Text("search-matches")
                        }),
                    ),
                    ("variant", V::Data(D::Text(operation))),
                    ("version", V::Data(D::Number(1.0))),
                    ("arguments", V::Object(&args)),
                ];
                let outer = [
                    (
                        "route",
                        V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
                    ),
                    ("request", V::Object(&request)),
                ];
                r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(if timed {
                        NativeTimingProfileV1::ProcessMonotonicV1
                    } else {
                        NativeTimingProfileV1::Off
                    });
                let q = normalize_literal_search_envelope_v1(
                    V::Object(&outer),
                    &s.context,
                    limits(),
                    &mut r,
                )
                .unwrap();
                let native = SearchRequest {
                    query: "a".into(),
                    scope: SearchScope::Captured(vec![ByteRange { start: 0, end: 6 }]),
                };
                let sf = binding::facts(&s);
                // Wrong operation selector refuses before evaluation; caller
                // retires its unconsumed preparation, then uses the correct one.
                let p = prepare(&mut o, &scope, generic, &mut r);
                let usage = o.source_search_usage();
                assert!(
                    o.query_discovered_bound_literal_search_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &context,
                            scope: &scope,
                            cut: &p,
                            entry,
                            variant: &descriptor.variants[(index + 1) % 3].reference,
                            facts: &discovery_support::facts()
                        },
                        &binding,
                        &q,
                        SearchReadFactsV1::Source(&sf),
                        &binding::qfacts(&native),
                        Some(true),
                        &mut r
                    )
                    .is_err()
                );
                assert_eq!(o.source_search_usage(), usage);
                o.retire_discovery_cut_v1(&p, &discovery_support::facts(), &mut r)
                    .unwrap();
                let p = prepare(&mut o, &scope, generic, &mut r);
                let result = o.query_discovered_bound_literal_search_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &context,
                        scope: &scope,
                        cut: &p,
                        entry,
                        variant: &descriptor.variants[index].reference,
                        facts: &discovery_support::facts(),
                    },
                    &binding,
                    &q,
                    SearchReadFactsV1::Source(&sf),
                    &binding::qfacts(&native),
                    Some(true),
                    &mut r,
                );
                if generic {
                    assert!(matches!(
                        result,
                        Err(DiscoveredLiteralSearchErrorV1::Discovery(
                            DiscoveryErrorV1::Unavailable
                        ))
                    ));
                    assert_eq!(o.source_search_usage(), usage);
                    o.retire_discovery_cut_v1(&p, &discovery_support::facts(), &mut r)
                        .unwrap();
                } else {
                    let mut result = result.unwrap();
                    let reply = o
                        .release_bound_literal_search_v1(
                            &result,
                            SearchReadFactsV1::Source(&sf),
                            &binding::qfacts(&native),
                            Some(true),
                            &mut r,
                        )
                        .unwrap();
                    assert_eq!(reply.generation, 0);
                    assert_eq!(reply.matching_profile, LITERAL_EXACT_MATCHING_PROFILE_V1);
                    let control = SourceReadContextControlV1 {
                        scope: &s,
                        owner_control_allowed: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    };
                    assert!(o.retire_bound_literal_search_v1(
                        &mut result,
                        SearchResultControlV1::Single(&control),
                        &mut r
                    ));
                }
                o.retire_logical_search_binding_v1(&binding, &mut r);
                o.stop_discovery_context_v1(&context, &scope, &discovery_support::facts(), &mut r)
                    .unwrap();
                o.retire_discovery_context_v1(&context, &discovery_support::facts(), &mut r)
                    .unwrap();
                o.stop_discovery_admission_v1(&discovery_support::facts(), &mut r)
                    .unwrap();
                o.retire_discovery_registry_v1(&discovery_support::facts(), &mut r)
                    .unwrap();
                assert_eq!(o.retention_usage().reserved, before);
                assert_eq!(o.source_read_context_usage().holds, 0);
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(!format!("{events:?}").contains("PRIVATE"));
                if capacity == 256 {
                    assert!(
                        events
                            .iter()
                            .any(|e| e.boundary == Boundary::DiscoveryLiteralSearchDispatch
                                && e.service_time.is_some() == timed)
                    );
                }
            }
        }
    }
}

#[test]
fn literal_search_outer_envelopes_are_closed_and_do_not_downgrade_or_reinterpret_roles() {
    let (o, c, mut r) = fixture();
    let actor = actor();
    let reference = [
        ("authority", D::Text("authority")),
        ("document", D::Text("logical")),
        ("incarnation", D::Number(7.0)),
        ("revision", D::Number(0.0)),
    ];
    let args = fields(
        SearchReadContextV1::Source(&c),
        "literal-matches",
        D::Object(&reference),
        "a",
        LITERAL_EXACT_MATCHING_PROFILE_V1,
        D::Array(&[]),
        D::Null,
    );
    let before = o.source_read_context_usage();
    for (route, family, version, variant) in [
        ("typed", 4.0, 1.0, "literal-matches"),
        ("unknown", 4.0, 1.0, "literal-matches"),
        ("typed", 5.0, 1.0, "literal-matches"),
        ("typed", 4.0, 2.0, "literal-matches"),
        ("typed", 4.0, 1.0, "next-match"),
        ("typed", 4.0, 1.0, "unknown-operation"),
    ] {
        let request = [
            ("family", V::Data(D::Number(family))),
            ("variant", V::Data(D::Text(variant))),
            ("version", V::Data(D::Number(version))),
            ("arguments", V::Object(&args)),
        ];
        let outer = [
            ("route", V::Data(D::Text(route))),
            ("request", V::Object(&request)),
        ];
        assert_eq!(
            normalize_literal_search_envelope_v1(V::Object(&outer), &actor, limits(), &mut r)
                .is_ok(),
            route == "typed" && family == 4.0 && version == 1.0 && variant == "literal-matches"
        );
        let extra = [outer[0], outer[1], ("extra", V::Data(D::Null))];
        assert!(
            normalize_literal_search_envelope_v1(V::Object(&extra), &actor, limits(), &mut r)
                .is_err()
        );
        let duplicate = [outer[0], outer[0]];
        assert!(
            normalize_literal_search_envelope_v1(V::Object(&duplicate), &actor, limits(), &mut r)
                .is_err()
        );
    }
    assert_eq!(o.source_read_context_usage(), before);
}
