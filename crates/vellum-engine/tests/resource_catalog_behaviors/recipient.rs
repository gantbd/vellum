use super::*;
fn read_facts() -> TrustedResourceCatalogReadFactsV1 {
    TrustedResourceCatalogReadFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        visibility_attested: Some(true),
        visibility_complete: Some(true),
        projection_order_attested: Some(true),
        registry_coverage_complete: Some(true),
        retention_ready: Some(true),
        transfer_ready: Some(true),
        registry_ready: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn given_registered_resources_when_a_recipient_inspects_then_only_permitted_metadata_is_released() {
    for route in [None, Some(true), Some(false)] {
        exercise(route);
    }
}
fn exercise(route: Option<bool>) {
    let mut o = if route.is_none() {
        owner()
    } else {
        SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 4096,
                source: Limits {
                    max_source_bytes: 1024,
                    max_edits: 16,
                    max_edit_payload_bytes: 4096,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 0,
                max_retained: 0,
                max_intent_bytes: 8192,
            },
        )
    };
    let mut r = Recorder::disabled();
    let source = o.create(b"prose and code", &mut r).unwrap();
    let read = address(ResourceCatalogTableV1::Read, "read");
    let write = address(ResourceCatalogTableV1::Write, "write");
    let initial = [
        ResourceCatalogRowV1 {
            address: address(ResourceCatalogTableV1::Read, "read"),
            generation: 0,
            configuration: configuration(ResourceCatalogTableV1::Read),
        },
        ResourceCatalogRowV1 {
            address: address(ResourceCatalogTableV1::Write, "write"),
            generation: 0,
            configuration: configuration(ResourceCatalogTableV1::Write),
        },
    ];
    o.configure_resource_catalog_v1(&initial, limits(), &facts(), &mut r)
        .unwrap();
    let scope = ResourceCatalogReadScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "a".into(),
            engine: "engine".into(),
            recipient: "sdk".into(),
            purpose: "inspect".into(),
            destination: "client".into(),
            profile: RESOURCE_CATALOG_READ_PROFILE_V1.into(),
            continuity: "session".into(),
        },
        policy: "approved".into(),
    };
    let mut l = ResourceCatalogReadLimitsV1 {
        max_contexts: 1,
        max_preparations: 1,
        max_rows: 4,
        max_selectors: 4,
        max_issued: 8,
        max_page_rows: 2,
        max_root_bytes: 1_048_576,
        max_packet_bytes: 65536,
        max_scope_bytes: 4096,
        max_capture_bytes: 16384,
        max_row_bytes: 4096,
        max_selector_bytes: 256,
        max_reply_bytes: 8192,
        max_pool_bytes: usize::MAX,
    };
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    o.configure_retention_budget(
        l.max_pool_bytes + resource_catalog_read_classification_bytes_v1() + 7 * 8192,
        &mut r,
    )
    .unwrap();
    o.configure_resource_catalog_reader_v1(&scope, l, &read_facts(), &mut r)
        .unwrap();
    // Order is host-approved visible order, not physical native catalog order.
    let visibility = [
        ResourceCatalogVisibilityV1 {
            address: &read,
            visible: true,
            descriptor: false,
            order: Some(1),
        },
        ResourceCatalogVisibilityV1 {
            address: &write,
            visible: true,
            descriptor: true,
            order: Some(0),
        },
    ];
    let p = o
        .prepare_resource_catalog_visibility_v1(&scope, &visibility, &read_facts(), &mut r)
        .unwrap();
    let c = o
        .acquire_resource_catalog_read_v1(&scope, &p, &read_facts(), &mut r)
        .unwrap();
    let before = (o.resource_usage(), o.generation(), o.retention_usage());
    let p = o
        .prepare_resource_catalog_visibility_v1(&scope, &visibility, &read_facts(), &mut r)
        .unwrap();
    let ResourceCatalogReadReplyV1::Page {
        context,
        entries,
        continuation,
    } = query(
        &mut o,
        route,
        &c,
        &scope,
        &p,
        ResourceCatalogReadQueryV1::List {
            limit: 2,
            continuation: None,
        },
        &read_facts(),
        &mut r,
    )
    else {
        panic!()
    };
    assert_eq!(context, c);
    assert!(continuation.is_none());
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].table, ResourceCatalogTableV1::Write);
    let mut owned = None;
    for (i, entry) in entries.iter().enumerate() {
        let p = o
            .prepare_resource_catalog_visibility_v1(&scope, &visibility, &read_facts(), &mut r)
            .unwrap();
        let out = query(
            &mut o,
            route,
            &c,
            &scope,
            &p,
            ResourceCatalogReadQueryV1::Inspect {
                entry: &entry.reference,
            },
            &read_facts(),
            &mut r,
        );
        assert_eq!(out.coverage(), "captured-permitted");
        let ResourceCatalogReadReplyV1::Registration {
            context,
            entry: actual,
            descriptor,
            ..
        } = out
        else {
            panic!()
        };
        assert_eq!(context, c);
        assert_eq!(actual, entry.reference);
        assert_eq!(descriptor.is_some(), i == 0);
        if descriptor.is_some() {
            owned = descriptor;
        }
    }
    assert_eq!(
        (o.resource_usage(), o.generation(), o.retention_usage()),
        before
    );
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "prose and code"
    );
    o.stop_resource_catalog_read_v1(&c, &scope, &read_facts(), &mut r)
        .unwrap();
    o.retire_resource_catalog_read_v1(&c, &read_facts(), &mut r)
        .unwrap();
    o.retire_resource_catalog_reader_v1(&read_facts(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(matches!(
        owned.unwrap(),
        ResourceCatalogReadDescriptorV1::Write { .. }
    ));
}

#[allow(clippy::too_many_arguments)]
fn query(
    o: &mut SourceCoordinator,
    route: Option<bool>,
    c: &ResourceCatalogReadContextIdV1,
    s: &ResourceCatalogReadScopeV1,
    cut: &ResourceCatalogVisibilityIdV1,
    query: ResourceCatalogReadQueryV1<'_>,
    facts: &TrustedResourceCatalogReadFactsV1,
    r: &mut Recorder,
) -> ResourceCatalogReadReplyV1 {
    let Some(typed) = route else {
        return o
            .query_resource_catalog_read_v1(c, s, cut, query, facts, r)
            .unwrap();
    };
    use ResourceCatalogReadEnvelopeValueV1 as V;
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    let list = matches!(query, ResourceCatalogReadQueryV1::List { .. });
    let q = [
        (
            "operation",
            V::Data(D::Text(if list { "list" } else { "inspect" })),
        ),
        (
            if list { "limit" } else { "entry" },
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
    let a = [
        ("contract", V::Data(D::Text("resource-catalog-query/1"))),
        ("operation", V::Data(D::Text("capabilities"))),
        ("context", V::Context(c)),
        (
            "profile",
            V::Data(D::Text(RESOURCE_CATALOG_READ_PROFILE_V1)),
        ),
        ("request", V::Object(&q[..if list { 3 } else { 2 }])),
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
        ("arguments", V::Object(&a)),
    ];
    let e = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&h)),
    ];
    o.query_decoded_resource_catalog_read_v1(
        &V::Object(&e),
        CaptureLimitsV1 {
            max_bytes: 8192,
            max_nodes: 128,
            max_depth: 8,
        },
        s,
        cut,
        facts,
        r,
    )
    .unwrap()
}
