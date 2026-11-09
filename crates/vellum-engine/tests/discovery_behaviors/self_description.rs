use super::*;
use vellum_engine::capture::{CapturedValueV1 as C, InertCaptureV1};

fn field<'a>(c: &'a InertCaptureV1, v: &'a C, name: &str) -> &'a C {
    let C::Object(rows) = v else { panic!("object") };
    c.node(rows.iter().find(|(k, _)| k == name).unwrap().1)
        .unwrap()
}
fn fixture() -> support::Fixture {
    let mut f = support::Fixture::new();
    let mut l = limits();
    l.metadata.max_nodes = 1024;
    l.metadata.max_depth = 12;
    l.metadata.max_bytes = 32768;
    l.max_row_bytes = 65536;
    l.max_capture_bytes = 131072;
    l.max_packet_bytes = 131072;
    l.max_reply_bytes = 131072;
    l.max_held_bytes = l.declared_bytes().unwrap();
    f.owner
        .configure_retention_budget(l.max_held_bytes + 262144, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_discovery_v1(&scope(), l, &facts(), &mut f.recorder)
        .unwrap();
    f
}

fn visible() -> NativeBuiltinEntryVisibilityV1<3> {
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
fn prepare(f: &mut support::Fixture) -> DiscoveryCutIdV1 {
    f.owner
        .prepare_native_discovery_query_cut_v1(&scope(), &visible(), &facts(), &mut f.recorder)
        .unwrap()
}
struct Installed {
    context: DiscoveryContextIdV1,
    entry: DiscoveryEntryIdV1,
    variants: Vec<DiscoveryVariantIdV1>,
}
fn install(f: &mut support::Fixture) -> Installed {
    let p = prepare(f);
    let context = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let p = prepare(f);
    let DiscoveryReplyV1::Page { entries, .. } = f
        .owner
        .query_discovery_v1(
            &context,
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
        panic!("page")
    };
    let entry = entries[0].reference.clone();
    let p = prepare(f);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = f
        .owner
        .query_discovery_v1(
            &context,
            &scope(),
            &p,
            DiscoveryQueryV1::Inspect { entry: &entry },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("descriptor")
    };
    Installed {
        context,
        entry,
        variants: descriptor
            .variants
            .into_iter()
            .map(|v| v.reference)
            .collect(),
    }
}
fn with_query<'h, T>(
    typed: bool,
    context: &'h DiscoveryContextIdV1,
    query: DiscoveryQueryV1<'h>,
    consume: impl for<'a> FnOnce(&'a DiscoveryEnvelopeValueV1<'a, 'h>) -> T,
) -> T {
    use DiscoveryEnvelopeValueV1 as V;
    let d = query.operation().declaration();
    let mut args = [
        ("contract", V::Data(D::Text(d.contract))),
        ("operation", V::Data(D::Text(query.operation().name()))),
        ("context", V::Context(context)),
        (
            "profile",
            V::Data(D::Text(OWNED_CONTRACT_DISCOVERY_PROFILE_V1)),
        ),
        ("", V::Data(D::Null)),
        ("", V::Data(D::Null)),
        ("", V::Data(D::Null)),
    ];
    match query {
        DiscoveryQueryV1::List {
            limit,
            continuation,
        } => {
            args[4] = ("limit", V::Data(D::Number(limit as f64)));
            args[5] = (
                "continuation",
                continuation.map_or(V::Data(D::Null), V::Cursor),
            );
        }
        DiscoveryQueryV1::Inspect { entry } => args[4] = ("entry", V::Entry(entry)),
        DiscoveryQueryV1::AdvisoryEnablement {
            entry,
            variant,
            target,
        } => {
            args[4] = ("entry", V::Entry(entry));
            args[5] = ("variant", V::Variant(variant));
            args[6] = ("target", V::Target(target));
        }
    }
    let request = [
        (
            if typed { "family" } else { "name" },
            V::Data(if typed {
                D::Number(d.family as f64)
            } else {
                D::Text(d.name)
            }),
        ),
        ("variant", V::Data(D::Text(query.operation().name()))),
        ("version", V::Data(D::Number(d.version as f64))),
        ("arguments", V::Object(&args[..d.fields.len()])),
    ];
    let outer = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&request)),
    ];
    consume(&V::Object(&outer))
}
#[allow(clippy::too_many_arguments)]
fn dispatch(
    f: &mut support::Fixture,
    b: &Installed,
    variant: usize,
    typed: bool,
    context: &DiscoveryContextIdV1,
    query: DiscoveryQueryV1<'_>,
    metadata_facts: &TrustedDiscoveryFactsV1,
    native_facts: &TrustedDiscoveryFactsV1,
) -> Result<DiscoveryReplyV1, DiscoveryErrorV1> {
    let route_cut = prepare(f);
    let native_cut = recognition::role_cut(f);
    let result = with_query(typed, context, query, |envelope| {
        f.owner.query_discovered_discovery_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &b.context,
                scope: &scope(),
                cut: &route_cut,
                entry: &b.entry,
                variant: &b.variants[variant],
                facts: metadata_facts,
            },
            context,
            &scope(),
            &native_cut,
            envelope,
            native_facts,
            &mut f.recorder,
        )
    });
    // Both cuts are independently caller owned; a pre-service refusal may leave
    // either unconsumed. Protected retirement accepts only a still-live cut.
    let _ = f
        .owner
        .retire_discovery_cut_v1(&route_cut, &facts(), &mut f.recorder);
    let _ = f
        .owner
        .retire_discovery_cut_v1(&native_cut, &facts(), &mut f.recorder);
    result
}
fn enable(f: &mut support::Fixture, context: &DiscoveryContextIdV1) {
    let mut l = DiscoveryDecoderLimitsV1 {
        input: CaptureLimitsV1 {
            max_bytes: 8192,
            max_nodes: 256,
            max_depth: 12,
        },
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.reserved_bytes().unwrap();
    f.owner
        .enable_discovery_decoder_v1(context, &scope(), l, &facts(), &mut f.recorder)
        .unwrap();
}
fn retire(f: &mut support::Fixture, contexts: &[&DiscoveryContextIdV1]) {
    for context in contexts {
        f.owner
            .stop_discovery_context_v1(context, &scope(), &facts(), &mut f.recorder)
            .unwrap();
        f.owner
            .retire_discovery_context_v1(context, &facts(), &mut f.recorder)
            .unwrap();
    }
    f.owner
        .stop_discovery_admission_v1(&facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_registry_v1(&facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
}
#[test]
fn installed_discovery_queries_use_original_native_handlers_on_both_routes_and_capture_modes() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for typed in [false, true] {
        for mode in 0..4 {
            let mut f = fixture();
            let b = install(&mut f);
            let roles = recognition::roles(&mut f);
            enable(&mut f, &roles.context);
            f.recorder = match mode {
                0 => Recorder::disabled(),
                1 => Recorder::try_with_capacity(512).unwrap(),
                2 => Recorder::try_with_capacity(512)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                _ => Recorder::try_with_capacity(1)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            };
            let generation = f.owner.generation();
            let held = f.owner.retention_usage();
            let page = dispatch(
                &mut f,
                &b,
                0,
                typed,
                &roles.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                &facts(),
                &facts(),
            )
            .unwrap();
            let DiscoveryReplyV1::Page {
                context,
                entries,
                continuation: Some(cursor),
                ..
            } = page
            else {
                panic!("page")
            };
            assert_eq!(context, roles.context);
            assert_eq!(entries[0].reference, roles.entry);
            assert_eq!(cursor, roles.cursor);
            let continued = dispatch(
                &mut f,
                &b,
                0,
                typed,
                &roles.context,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: Some(&cursor),
                },
                &facts(),
                &facts(),
            )
            .unwrap();
            assert!(matches!(
                continued,
                DiscoveryReplyV1::Page {
                    continuation: None,
                    ..
                }
            ));
            let descriptor = dispatch(
                &mut f,
                &b,
                1,
                typed,
                &roles.context,
                DiscoveryQueryV1::Inspect {
                    entry: &roles.entry,
                },
                &facts(),
                &facts(),
            )
            .unwrap();
            let DiscoveryReplyV1::Descriptor { descriptor, .. } = descriptor else {
                panic!("descriptor")
            };
            assert_eq!(descriptor.reference, roles.entry);
            assert!(descriptor.variants[0].schema.is_some());
            assert!(descriptor.presentation.is_some());
            let advice = dispatch(
                &mut f,
                &b,
                2,
                typed,
                &roles.context,
                DiscoveryQueryV1::AdvisoryEnablement {
                    entry: &roles.entry,
                    variant: &roles.variant,
                    target: &roles.target,
                },
                &facts(),
                &facts(),
            )
            .unwrap();
            assert!(
                matches!(advice, DiscoveryReplyV1::Advice { context, entry, variant, target, enabled: false }
                if context == roles.context && entry == roles.entry && variant == roles.variant && target == roles.target)
            );
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.retention_usage(), held);
            assert_eq!(f.owner.discovery_usage_v1().unwrap().contexts, 2);
            let mut dispatched = 0;
            while let Some(o) = f.recorder.pop_front() {
                if o.boundary == Boundary::DiscoveryQueryDispatch {
                    dispatched += 1;
                    assert_eq!(o.service_time.is_some(), mode == 2);
                }
            }
            if mode == 1 || mode == 2 {
                assert_eq!(dispatched, 4);
            }
            retire(&mut f, &[&b.context, &roles.context]);
        }
    }
}
#[test]
fn metadata_cannot_enable_a_decoder_or_override_native_permission_and_wrong_selector_cannot_apply_loss()
 {
    for typed in [false, true] {
        let mut f = fixture();
        let b = install(&mut f);
        let roles = recognition::roles(&mut f);
        let q = DiscoveryQueryV1::List {
            limit: 1,
            continuation: None,
        };
        assert_eq!(
            dispatch(&mut f, &b, 0, typed, &roles.context, q, &facts(), &facts()).unwrap_err(),
            DiscoveryErrorV1::Unavailable
        );
        assert!(f.owner.discovery_decoder_usage_v1(&roles.context).is_err());
        enable(&mut f, &roles.context);
        let mut loss = facts();
        loss.authorized = Some(false);
        assert_eq!(
            dispatch(&mut f, &b, 1, typed, &roles.context, q, &loss, &loss).unwrap_err(),
            DiscoveryErrorV1::Unavailable
        );
        assert!(
            !f.owner
                .discovery_context_usage_v1(&b.context)
                .unwrap()
                .stopped
        );
        assert!(
            !f.owner
                .discovery_context_usage_v1(&roles.context)
                .unwrap()
                .stopped
        );
        assert!(dispatch(&mut f, &b, 0, typed, &roles.context, q, &facts(), &facts()).is_ok());
        assert_eq!(
            dispatch(&mut f, &b, 0, typed, &roles.context, q, &facts(), &loss).unwrap_err(),
            DiscoveryErrorV1::Unavailable
        );
        assert!(
            f.owner
                .discovery_context_usage_v1(&roles.context)
                .unwrap()
                .stopped
        );
        assert!(
            !f.owner
                .discovery_context_usage_v1(&b.context)
                .unwrap()
                .stopped
        );
        retire(&mut f, &[&b.context, &roles.context]);
    }
}
#[test]
fn self_description_refusal_does_not_grow_capacity_or_disable_an_affordable_existing_source() {
    let mut f = setup();
    let before = f.owner.discovery_usage_v1().unwrap();
    let held = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .prepare_native_discovery_query_cut_v1(&scope(), &visible(), &facts(), &mut f.recorder,)
            .unwrap_err(),
        DiscoveryErrorV1::Limit
    );
    assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
    assert_eq!(f.owner.retention_usage(), held);
    let a = acquire(&mut f, true, true);
    let _ = entry(&mut f, &a.context, true);
    retire(&mut f, &[&a.context]);
}
#[test]
fn self_description_captured_schema_ceiling_cannot_be_widened_by_later_visible_metadata() {
    let mut f = fixture();
    let mut hidden = visible();
    hidden.schema = false;
    let p = f
        .owner
        .prepare_native_discovery_query_cut_v1(&scope(), &hidden, &facts(), &mut f.recorder)
        .unwrap();
    let a = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap();
    let p = prepare(&mut f);
    let DiscoveryReplyV1::Page { entries, .. } = f
        .owner
        .query_discovery_v1(
            &a.context,
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
        panic!("page")
    };
    let p = prepare(&mut f);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = f
        .owner
        .query_discovery_v1(
            &a.context,
            &scope(),
            &p,
            DiscoveryQueryV1::Inspect {
                entry: &entries[0].reference,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("descriptor")
    };
    assert_eq!(descriptor.variants.len(), 3);
    assert!(descriptor.variants.iter().all(|v| v.schema.is_none()));
    assert!(descriptor.presentation.is_none());
    retire(&mut f, &[&a.context]);
}
#[test]
fn discovery_describes_its_actual_three_queries_without_acquiring_another_context() {
    let mut f = fixture();
    let before = f.owner.discovery_usage_v1().unwrap();
    let p = prepare(&mut f);
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().contexts,
        before.contexts
    );
    let a = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap();
    let p = prepare(&mut f);
    let DiscoveryReplyV1::Page { entries, .. } = f
        .owner
        .query_discovery_v1(
            &a.context,
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
        panic!("page")
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, DiscoveryExecutionKindV1::Query);
    let p = prepare(&mut f);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = f
        .owner
        .query_discovery_v1(
            &a.context,
            &scope(),
            &p,
            DiscoveryQueryV1::Inspect {
                entry: &entries[0].reference,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("descriptor")
    };
    assert_eq!(descriptor.variants.len(), 3);
    for (v, declaration) in descriptor
        .variants
        .iter()
        .zip(native_discovery_input_forms_v1())
    {
        let schema = v.schema.as_ref().unwrap();
        let input = field(schema, schema.root(), "input");
        let C::Object(roles) = field(schema, input, "fields") else {
            panic!("roles")
        };
        assert_eq!(
            roles.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            declaration.fields
        );
        let requirements = field(schema, schema.root(), "requirements");
        let object = field(schema, requirements, "schema");
        assert!(matches!(
            field(schema, object, "additionalProperties"),
            C::Boolean(false)
        ));
        let C::Array(required) = field(schema, object, "required") else {
            panic!("required")
        };
        assert_eq!(required.len(), declaration.fields.len());
        for (i, key) in required.iter().zip(declaration.fields) {
            assert!(matches!(schema.node(*i), Some(C::Text(v)) if v == key));
        }
        let outcomes = field(schema, schema.root(), "outcomes");
        assert!(
            matches!(field(schema, outcomes, "nativeForm"), C::Text(n) if n == declaration.operation.result_declaration().name)
        );
    }
    f.owner
        .stop_discovery_context_v1(&a.context, &scope(), &facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_context_v1(&a.context, &facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .stop_discovery_admission_v1(&facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_registry_v1(&facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
}
