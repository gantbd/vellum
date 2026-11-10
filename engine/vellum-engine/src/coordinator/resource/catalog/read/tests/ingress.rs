use super::*;
use crate::capture::{CaptureLimitsV1, InertValueV1 as D};
use crate::coordinator::resource::catalog::read::ingress::Probe;
use ResourceCatalogReadEnvelopeValueV1 as V;
pub(in crate::coordinator) fn input_limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 128,
        max_depth: 8,
    }
}
pub(in crate::coordinator) fn request<'h, T>(
    context: &'h ResourceCatalogReadContextIdV1,
    typed: bool,
    query: ResourceCatalogReadQueryV1<'h>,
    consume: impl FnOnce(V<'_, 'h>) -> T,
) -> T {
    let op = match query {
        ResourceCatalogReadQueryV1::List { .. } => "list",
        ResourceCatalogReadQueryV1::Inspect { .. } => "inspect",
    };
    let nested = [
        ("operation", V::Data(D::Text(op))),
        (
            if op == "list" { "limit" } else { "entry" },
            match query {
                ResourceCatalogReadQueryV1::List { limit, .. } => V::Data(D::Number(limit as f64)),
                ResourceCatalogReadQueryV1::Inspect { entry } => V::Entry(entry),
            },
        ),
        (
            "continuation",
            match query {
                ResourceCatalogReadQueryV1::List {
                    continuation: Some(c),
                    ..
                } => V::Continuation(c),
                _ => V::Data(D::Null),
            },
        ),
    ];
    let args = [
        ("contract", V::Data(D::Text("resource-catalog-query/1"))),
        ("operation", V::Data(D::Text("capabilities"))),
        ("context", V::Context(context)),
        (
            "profile",
            V::Data(D::Text(RESOURCE_CATALOG_READ_PROFILE_V1)),
        ),
        (
            "request",
            V::Object(&nested[..if op == "list" { 3 } else { 2 }]),
        ),
    ];
    let h = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(D::Number(6.0))
            } else {
                V::Data(D::Text("inspect-context"))
            },
        ),
        ("variant", V::Data(D::Text("capabilities"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let e = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&h)),
    ];
    consume(V::Object(&e))
}
fn ready() -> (SourceCoordinator, Recorder) {
    let l = limits();
    let (mut o, r) = setup_with_parent(
        l,
        l.max_pool_bytes
            + resource_catalog_read_classification_bytes_v1()
            + 7 * input_limits().max_bytes,
    );
    o.set_settings_parent_limit_for_test(input_limits().max_bytes);
    (o, r)
}
#[test]
fn both_routes_list_continue_inspect_and_retry_with_original_bindings_and_exact_refund() {
    for typed in [false, true] {
        let (mut o, mut r) = ready();
        let c = acquire(&mut o, true, &mut r);
        let query = ResourceCatalogReadQueryV1::List {
            limit: 1,
            continuation: None,
        };
        let p = cut(&mut o, true, false, &mut r);
        let before = (o.retention_usage(), o.generation());
        let ResourceCatalogReadReplyV1::Page {
            context,
            entries,
            continuation,
        } = request(&c, typed, query, |e| {
            let n = normalize_resource_catalog_read_envelope_v1(
                &e,
                &scope(),
                input_limits(),
                8192,
                &mut r,
            )
            .unwrap();
            assert_eq!(n.is_typed_route(), typed);
            o.query_decoded_resource_catalog_read_v1(
                &e,
                input_limits(),
                &scope(),
                &p,
                &facts(),
                &mut r,
            )
        })
        .unwrap()
        else {
            panic!()
        };
        assert_eq!(context, c);
        assert_eq!(entries.len(), 1);
        assert!(continuation.is_some());
        assert_eq!((o.retention_usage(), o.generation()), before);
        let issued = o.resource_catalog_read_usage_v1().issued_lifetime;
        let p = cut(&mut o, true, false, &mut r);
        let ResourceCatalogReadReplyV1::Page {
            entries: retry,
            continuation: again,
            ..
        } = request(&c, typed, query, |e| {
            o.query_decoded_resource_catalog_read_v1(
                &e,
                input_limits(),
                &scope(),
                &p,
                &facts(),
                &mut r,
            )
        })
        .unwrap()
        else {
            panic!()
        };
        assert_eq!(retry[0].reference, entries[0].reference);
        assert_eq!(again, continuation);
        assert_eq!(o.resource_catalog_read_usage_v1().issued_lifetime, issued);
        let p = cut(&mut o, true, false, &mut r);
        let ResourceCatalogReadReplyV1::Page { entries: next, .. } = request(
            &c,
            typed,
            ResourceCatalogReadQueryV1::List {
                limit: 1,
                continuation: continuation.as_ref(),
            },
            |e| {
                o.query_decoded_resource_catalog_read_v1(
                    &e,
                    input_limits(),
                    &scope(),
                    &p,
                    &facts(),
                    &mut r,
                )
            },
        )
        .unwrap() else {
            panic!()
        };
        assert_ne!(next[0].reference, entries[0].reference);
        let p = cut(&mut o, true, false, &mut r);
        let ResourceCatalogReadReplyV1::Registration {
            context,
            entry,
            descriptor,
            ..
        } = request(
            &c,
            typed,
            ResourceCatalogReadQueryV1::Inspect {
                entry: &entries[0].reference,
            },
            |e| {
                o.query_decoded_resource_catalog_read_v1(
                    &e,
                    input_limits(),
                    &scope(),
                    &p,
                    &facts(),
                    &mut r,
                )
            },
        )
        .unwrap()
        else {
            panic!()
        };
        assert_eq!(context, c);
        assert_eq!(entry, entries[0].reference);
        assert!(descriptor.is_some());
        o.stop_resource_catalog_read_v1(&c, &scope(), &facts(), &mut r)
            .unwrap();
        o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
            .unwrap();
        o.retire_resource_catalog_reader_v1(&facts(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn malformed_nested_roles_and_profile_confusion_cannot_attribute_supplied_loss() {
    let (mut o, mut r) = ready();
    let c = acquire(&mut o, true, &mut r);
    let p = cut(&mut o, true, false, &mut r);
    let mut loss = facts();
    loss.continuous = Some(false);
    for case in 0..11 {
        let mut nested = vec![
            ("operation", V::Data(D::Text("list"))),
            ("limit", V::Data(D::Number(1.0))),
            ("continuation", V::Data(D::Null)),
        ];
        match case {
            0 => nested[0].1 = V::Data(D::Boolean(true)),
            1 => nested[0].1 = V::Data(D::Text("unknown")),
            2 => nested[1].1 = V::Data(D::Number(0.0)),
            3 => nested[1].1 = V::Data(D::Number(f64::NAN)),
            4 => nested[1].1 = V::Data(D::Text("1")),
            5 => nested[2].1 = V::Context(&c),
            6 => nested.push(("extra", V::Data(D::Null))),
            7 => nested[1].0 = "operation",
            _ => {}
        }
        let args = [
            (
                "contract",
                V::Data(D::Text(if case == 8 {
                    "configuration-query/1"
                } else {
                    "resource-catalog-query/1"
                })),
            ),
            ("operation", V::Data(D::Text("capabilities"))),
            (
                "context",
                if case == 9 {
                    V::Data(D::Object(&[]))
                } else {
                    V::Context(&c)
                },
            ),
            (
                "profile",
                V::Data(D::Text(if case == 10 {
                    "native-engine-settings-read/1"
                } else {
                    RESOURCE_CATALOG_READ_PROFILE_V1
                })),
            ),
            ("request", V::Object(&nested)),
        ];
        let h = [
            ("family", V::Data(D::Number(6.0))),
            ("variant", V::Data(D::Text("capabilities"))),
            ("version", V::Data(D::Number(1.0))),
            ("arguments", V::Object(&args)),
        ];
        let e = [
            ("route", V::Data(D::Text("typed"))),
            ("request", V::Object(&h)),
        ];
        let before = (o.retention_usage(), o.resource_catalog_read_usage_v1());
        assert!(
            o.query_decoded_resource_catalog_read_v1(
                &V::Object(&e),
                input_limits(),
                &scope(),
                &p,
                &loss,
                &mut r
            )
            .is_err()
        );
        assert_eq!(
            (o.retention_usage(), o.resource_catalog_read_usage_v1()),
            before
        );
        assert!(
            !o.catalog_read_pool().unwrap().contexts[c.slot]
                .as_ref()
                .unwrap()
                .stopped
        );
    }
    request(
        &c,
        true,
        ResourceCatalogReadQueryV1::List {
            limit: 1,
            continuation: None,
        },
        |e| {
            assert!(
                o.query_decoded_resource_catalog_read_v1(
                    &e,
                    input_limits(),
                    &scope(),
                    &p,
                    &loss,
                    &mut r
                )
                .is_err()
            )
        },
    );
    assert!(
        o.catalog_read_pool().unwrap().contexts[c.slot]
            .as_ref()
            .unwrap()
            .invalidated
    );
}
#[test]
fn actual_parent_intent_intersection_and_exact_workspace_headroom_are_independent() {
    let l = limits();
    let temporary = resource_catalog_read_classification_bytes_v1() + 7 * input_limits().max_bytes;
    for delta in [-1isize, 0, 1] {
        let (mut o, mut r) = setup_with_parent(
            l,
            l.max_pool_bytes + temporary.checked_add_signed(delta).unwrap(),
        );
        o.set_settings_parent_limit_for_test(8192);
        let c = acquire(&mut o, true, &mut r);
        let p = cut(&mut o, true, false, &mut r);
        let before = o.retention_usage();
        request(
            &c,
            true,
            ResourceCatalogReadQueryV1::List {
                limit: 1,
                continuation: None,
            },
            |e| {
                assert_eq!(
                    o.query_decoded_resource_catalog_read_v1(
                        &e,
                        input_limits(),
                        &scope(),
                        &p,
                        &facts(),
                        &mut r
                    )
                    .is_ok(),
                    delta >= 0
                )
            },
        );
        assert_eq!(o.retention_usage(), before);
        assert_eq!(
            o.resource_catalog_read_decoder_usage_v1(),
            ResourceCatalogReadDecoderUsageV1::default()
        );
    }
    let (mut o, mut r) = ready();
    let c = acquire(&mut o, true, &mut r);
    let p = cut(&mut o, true, false, &mut r);
    o.set_settings_parent_limit_for_test(0);
    let before = (o.retention_usage(), o.resource_catalog_read_usage_v1());
    request(
        &c,
        true,
        ResourceCatalogReadQueryV1::List {
            limit: 1,
            continuation: None,
        },
        |e| {
            assert!(
                o.query_decoded_resource_catalog_read_v1(
                    &e,
                    input_limits(),
                    &scope(),
                    &p,
                    &facts(),
                    &mut r
                )
                .is_err()
            )
        },
    );
    assert_eq!(
        (o.retention_usage(), o.resource_catalog_read_usage_v1()),
        before
    );
    // A fitting parent intent lower than the requested input ceiling still works.
    o.set_settings_parent_limit_for_test(4096);
    request(
        &c,
        false,
        ResourceCatalogReadQueryV1::List {
            limit: 1,
            continuation: None,
        },
        |e| {
            o.query_decoded_resource_catalog_read_v1(
                &e,
                input_limits(),
                &scope(),
                &p,
                &facts(),
                &mut r,
            )
        },
    )
    .unwrap();
}
#[test]
fn all_four_decoder_fault_and_reentry_stages_preserve_committed_aliases_and_protected_cleanup() {
    for point in 0..4 {
        for panic in [false, true] {
            let (mut o, mut r) = ready();
            let c = acquire(&mut o, true, &mut r);
            let p = cut(&mut o, true, false, &mut r);
            o.catalog_read_pool_mut().unwrap().decode_probe = Some(Probe { point, panic });
            let before = (
                o.generation(),
                o.retention_usage(),
                o.resource_catalog_read_usage_v1().issued_lifetime,
            );
            request(
                &c,
                true,
                ResourceCatalogReadQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                |e| {
                    assert_eq!(
                        o.query_decoded_resource_catalog_read_v1(
                            &e,
                            input_limits(),
                            &scope(),
                            &p,
                            &facts(),
                            &mut r
                        )
                        .is_ok(),
                        !panic
                    )
                },
            );
            assert_eq!((o.generation(), o.retention_usage()), (before.0, before.1));
            assert!(!o.faulted);
            assert_eq!(
                o.resource_catalog_read_decoder_usage_v1(),
                ResourceCatalogReadDecoderUsageV1 {
                    busy: false,
                    declared_workspace_bytes: 0,
                    faulted: panic
                }
            );
            assert_eq!(
                o.resource_catalog_read_usage_v1().issued_lifetime,
                before.2 + if !panic || point == 3 { 2 } else { 0 }
            );
            if panic {
                assert!(
                    o.prepare_resource_catalog_visibility_v1(&scope(), &[], &facts(), &mut r)
                        .is_err()
                );
                assert!(
                    o.acquire_resource_catalog_read_v1(&scope(), &p, &facts(), &mut r)
                        .is_err()
                );
                assert!(
                    o.query_resource_catalog_read_v1(
                        &c,
                        &scope(),
                        &p,
                        ResourceCatalogReadQueryV1::List {
                            limit: 1,
                            continuation: None
                        },
                        &facts(),
                        &mut r
                    )
                    .is_err()
                );
                if point != 3 {
                    o.retire_resource_catalog_visibility_v1(&p, &facts(), &mut r)
                        .unwrap();
                }
            } else {
                assert_eq!(
                    o.catalog_read_pool_mut().unwrap().decode_probe_report,
                    Some((true, true, true))
                );
            }
            o.stop_resource_catalog_read_v1(&c, &scope(), &facts(), &mut r)
                .unwrap();
            o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
                .unwrap();
            o.retire_resource_catalog_reader_v1(&facts(), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 0);
        }
    }
}

#[test]
fn busy_fence_protects_an_otherwise_empty_registry_and_complete_reply_guards_bind_values() {
    let (mut o, mut r) = ready();
    let c = acquire(&mut o, true, &mut r);
    let p = cut(&mut o, true, false, &mut r);
    let query = ResourceCatalogReadQueryV1::List {
        limit: 1,
        continuation: None,
    };
    let mut reply = request(&c, true, query, |e| {
        o.query_decoded_resource_catalog_read_v1(&e, input_limits(), &scope(), &p, &facts(), &mut r)
    })
    .unwrap();
    assert!(o.catalog_reply_bound(&c, query, &reply).unwrap());
    let ResourceCatalogReadReplyV1::Page { entries, .. } = &mut reply else {
        panic!()
    };
    entries[0].table = ResourceCatalogTableV1::Write;
    assert!(!o.catalog_reply_bound(&c, query, &reply).unwrap());
    o.stop_resource_catalog_read_v1(&c, &scope(), &facts(), &mut r)
        .unwrap();
    o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
        .unwrap();
    assert_eq!(o.resource_catalog_read_usage_v1().contexts, 0);
    o.catalog_read_pool_mut().unwrap().decode_probe = Some(Probe {
        point: 1,
        panic: false,
    });
    request(&c, true, query, |e| {
        assert!(
            o.query_decoded_resource_catalog_read_v1(
                &e,
                input_limits(),
                &scope(),
                &p,
                &facts(),
                &mut r
            )
            .is_err()
        )
    });
    assert_eq!(
        o.catalog_read_pool_mut().unwrap().decode_probe_report,
        Some((true, true, true))
    );
    assert_eq!(
        o.resource_catalog_read_decoder_usage_v1(),
        ResourceCatalogReadDecoderUsageV1::default()
    );
    o.retire_resource_catalog_reader_v1(&facts(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn normalization_respects_each_independent_native_boundary_and_does_not_traverse_selectors() {
    let (mut o, mut r) = ready();
    let c = acquire(&mut o, true, &mut r);
    for field in 0..3 {
        let mut low = 0;
        let mut high = match field {
            0 => 8192,
            1 => 128,
            _ => 8,
        };
        let valid = |bound, r: &mut Recorder| {
            let mut l = input_limits();
            match field {
                0 => l.max_bytes = bound,
                1 => l.max_nodes = bound,
                _ => l.max_depth = bound,
            }
            request(
                &c,
                false,
                ResourceCatalogReadQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                |e| normalize_resource_catalog_read_envelope_v1(&e, &scope(), l, 8192, r),
            )
            .is_ok()
        };
        assert!(valid(high, &mut r));
        while low < high {
            let middle = low + (high - low) / 2;
            if valid(middle, &mut r) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(high > 0);
        assert!(!valid(high - 1, &mut r));
        assert!(valid(high, &mut r));
        assert!(valid(high + 1, &mut r));
    }
    let ResourceCatalogReadReplyV1::Page { entries, .. } = page(&mut o, &c, 3, None, &mut r) else {
        panic!()
    };
    let n = request(
        &c,
        true,
        ResourceCatalogReadQueryV1::Inspect {
            entry: &entries[0].reference,
        },
        |e| normalize_resource_catalog_read_envelope_v1(&e, &scope(), input_limits(), 8192, &mut r),
    )
    .unwrap();
    assert!(
        matches!(n.query(), ResourceCatalogReadQueryV1::Inspect { entry } if entry == &entries[0].reference)
    );
}
