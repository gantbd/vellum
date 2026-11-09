use super::super::tests as catalog;
use super::*;
use crate::instrumentation::NativeTimingProfileV1;
pub(in crate::coordinator) mod ingress;
pub(in crate::coordinator) fn limits() -> ResourceCatalogReadLimitsV1 {
    let mut l = ResourceCatalogReadLimitsV1 {
        max_contexts: 4,
        max_preparations: 2,
        max_rows: 8,
        max_selectors: 16,
        max_issued: 64,
        max_page_rows: 4,
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
    l
}
pub(in crate::coordinator) fn scope() -> ResourceCatalogReadScopeV1 {
    ResourceCatalogReadScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "a".into(),
            engine: "engine".into(),
            recipient: "reader".into(),
            purpose: "catalog".into(),
            destination: "client".into(),
            profile: RESOURCE_CATALOG_READ_PROFILE_V1.into(),
            continuity: "session".into(),
        },
        policy: "p1".into(),
    }
}
pub(in crate::coordinator) fn facts() -> TrustedResourceCatalogReadFactsV1 {
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
fn setup(l: ResourceCatalogReadLimitsV1) -> (SourceCoordinator, Recorder) {
    setup_with_parent(l, l.max_pool_bytes)
}
pub(in crate::coordinator) fn setup_with_parent(
    l: ResourceCatalogReadLimitsV1,
    maximum: usize,
) -> (SourceCoordinator, Recorder) {
    let mut o = catalog::bare();
    let mut r = Recorder::disabled();
    let initial: Vec<_> = [
        (ResourceCatalogTableV1::Read, "a"),
        (ResourceCatalogTableV1::Read, "b"),
        (ResourceCatalogTableV1::Write, "c"),
    ]
    .into_iter()
    .map(|(table, id)| ResourceCatalogRowV1 {
        address: catalog::address(table, id),
        generation: 0,
        configuration: catalog::config(table, id),
    })
    .collect();
    o.configure_resource_catalog_v1(&initial, catalog::limits(), &catalog::facts(), &mut r)
        .unwrap();
    o.configure_retention_budget(maximum, &mut r).unwrap();
    o.configure_resource_catalog_reader_v1(&scope(), l, &facts(), &mut r)
        .unwrap();
    (o, r)
}
pub(in crate::coordinator) fn cut(
    o: &mut SourceCoordinator,
    descriptors: bool,
    reverse: bool,
    r: &mut Recorder,
) -> ResourceCatalogVisibilityIdV1 {
    let (root, _) = o.resources.catalog.current().unwrap();
    let addresses: Vec<_> = root
        .rows
        .iter()
        .map(|row| row.address.copy().unwrap())
        .collect();
    let visible: Vec<_> = addresses
        .iter()
        .enumerate()
        .map(|(i, address)| ResourceCatalogVisibilityV1 {
            address,
            visible: true,
            descriptor: descriptors,
            order: Some(if reverse { addresses.len() - 1 - i } else { i }),
        })
        .collect();
    o.prepare_resource_catalog_visibility_v1(&scope(), &visible, &facts(), r)
        .unwrap()
}
pub(in crate::coordinator) fn acquire(
    o: &mut SourceCoordinator,
    descriptors: bool,
    r: &mut Recorder,
) -> ResourceCatalogReadContextIdV1 {
    let p = cut(o, descriptors, false, r);
    o.acquire_resource_catalog_read_v1(&scope(), &p, &facts(), r)
        .unwrap()
}
fn page(
    o: &mut SourceCoordinator,
    c: &ResourceCatalogReadContextIdV1,
    limit: usize,
    after: Option<&ResourceCatalogReadContinuationIdV1>,
    r: &mut Recorder,
) -> ResourceCatalogReadReplyV1 {
    let p = cut(o, true, false, r);
    o.query_resource_catalog_read_v1(
        c,
        &scope(),
        &p,
        ResourceCatalogReadQueryV1::List {
            limit,
            continuation: after,
        },
        &facts(),
        r,
    )
    .unwrap()
}
fn mutate(
    o: &mut SourceCoordinator,
    id: &str,
    configuration: ResourceCatalogConfigurationV1,
    r: &mut Recorder,
) {
    let address = catalog::address(
        if id == "c" {
            ResourceCatalogTableV1::Write
        } else {
            ResourceCatalogTableV1::Read
        },
        id,
    );
    let generation = o
        .resources
        .catalog
        .current()
        .unwrap()
        .0
        .rows
        .iter()
        .find(|row| row.address == address)
        .unwrap()
        .generation;
    let p = o
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address,
                expected: CatalogExpectationV1::Present { generation },
                action: ResourceCatalogActionV1::Install(configuration),
            }],
            &catalog::facts(),
            r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &catalog::facts(), r)
        .unwrap();
}
#[test]
fn pages_retry_stably_paginate_inspect_all_descriptor_types_and_clean_up() {
    let (mut o, mut r) = setup(limits());
    let c = acquire(&mut o, true, &mut r);
    let ResourceCatalogReadReplyV1::Page {
        context,
        entries,
        continuation,
    } = page(&mut o, &c, 1, None, &mut r)
    else {
        panic!()
    };
    assert_eq!(context, c);
    assert_eq!(entries.len(), 1);
    let issued = o.resource_catalog_read_usage_v1().issued_lifetime;
    let ResourceCatalogReadReplyV1::Page {
        entries: retry,
        continuation: same,
        ..
    } = page(&mut o, &c, 1, None, &mut r)
    else {
        panic!()
    };
    assert_eq!(entries[0].reference, retry[0].reference);
    assert_eq!(continuation, same);
    assert_eq!(o.resource_catalog_read_usage_v1().issued_lifetime, issued);
    let ResourceCatalogReadReplyV1::Page {
        entries: second,
        continuation: next,
        ..
    } = page(&mut o, &c, 1, continuation.as_ref(), &mut r)
    else {
        panic!()
    };
    let ResourceCatalogReadReplyV1::Page {
        entries: last,
        continuation: end,
        ..
    } = page(&mut o, &c, 1, next.as_ref(), &mut r)
    else {
        panic!()
    };
    assert!(end.is_none());
    assert_eq!(last[0].table, ResourceCatalogTableV1::Write);
    for entry in [
        &entries[0].reference,
        &second[0].reference,
        &last[0].reference,
    ] {
        let p = cut(&mut o, true, false, &mut r);
        let out = o
            .query_resource_catalog_read_v1(
                &c,
                &scope(),
                &p,
                ResourceCatalogReadQueryV1::Inspect { entry },
                &facts(),
                &mut r,
            )
            .unwrap();
        assert_eq!(out.coverage(), "captured-permitted");
        assert!(!format!("{out:?}").contains("PRIVATE_LOCATOR"));
        let ResourceCatalogReadReplyV1::Registration {
            context,
            entry: actual,
            table,
            descriptor,
        } = out
        else {
            panic!()
        };
        assert_eq!(context, c);
        assert_eq!(&actual, entry);
        match (table, descriptor.unwrap()) {
            (
                ResourceCatalogTableV1::Read,
                ResourceCatalogReadDescriptorV1::StrictUtf8OneBom {
                    insertion_line_break: CatalogInsertionLineBreakV1::CrLf,
                    ..
                },
            ) => {}
            (
                ResourceCatalogTableV1::Write,
                ResourceCatalogReadDescriptorV1::Write {
                    identity_known: false,
                    procedure,
                    ..
                },
            ) => assert_eq!(procedure, "staged-replacement"),
            _ => panic!("wrong descriptor"),
        }
    }
    o.stop_resource_catalog_read_v1(&c, &scope(), &facts(), &mut r)
        .unwrap();
    o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
        .unwrap();
    let issued = o.resource_catalog_read_usage_v1().issued_lifetime;
    o.retire_resource_catalog_reader_v1(&facts(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.resource_catalog_read_usage_v1().issued_lifetime, issued);
    assert!(
        o.configure_resource_catalog_reader_v1(&scope(), limits(), &facts(), &mut r)
            .is_err()
    );
}
#[test]
fn private_changes_and_new_descriptor_permission_do_not_widen_or_stale_table_only_captures() {
    let (mut o, mut r) = setup(limits());
    let c = acquire(&mut o, false, &mut r);
    let ResourceCatalogReadReplyV1::Page { entries, .. } = page(&mut o, &c, 3, None, &mut r) else {
        panic!()
    };
    let mut changed = catalog::config(ResourceCatalogTableV1::Read, "DIFFERENT_PRIVATE_KEY");
    changed.locator = "another-private-location".into();
    changed.capabilities = ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read: true };
    mutate(&mut o, "a", changed, &mut r);
    let p = cut(&mut o, true, true, &mut r);
    let ResourceCatalogReadReplyV1::Registration { descriptor, .. } = o
        .query_resource_catalog_read_v1(
            &c,
            &scope(),
            &p,
            ResourceCatalogReadQueryV1::Inspect {
                entry: &entries[0].reference,
            },
            &facts(),
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(descriptor.is_none());
    let fresh = acquire(&mut o, true, &mut r);
    let ResourceCatalogReadReplyV1::Page {
        entries: fresh_entries,
        ..
    } = page(&mut o, &fresh, 3, None, &mut r)
    else {
        panic!()
    };
    let p = cut(&mut o, true, false, &mut r);
    assert!(matches!(
        o.query_resource_catalog_read_v1(
            &fresh,
            &scope(),
            &p,
            ResourceCatalogReadQueryV1::Inspect {
                entry: &fresh_entries[0].reference
            },
            &facts(),
            &mut r
        )
        .unwrap(),
        ResourceCatalogReadReplyV1::Registration {
            descriptor: Some(ResourceCatalogReadDescriptorV1::AnnotationsV1 { atomic_read: true }),
            ..
        }
    ));
}
#[test]
fn changed_permitted_values_invalidate_absorbingly_and_foreign_context_scope_cannot_attribute_loss()
{
    let (mut o, mut r) = setup(limits());
    let c = acquire(&mut o, true, &mut r);
    let sibling = acquire(&mut o, true, &mut r);
    let mut loss = facts();
    loss.continuous = Some(false);
    let mut foreign = scope();
    foreign.policy.push_str("-foreign");
    let p = cut(&mut o, true, false, &mut r);
    assert!(
        o.query_resource_catalog_read_v1(
            &c,
            &foreign,
            &p,
            ResourceCatalogReadQueryV1::List {
                limit: 1,
                continuation: None
            },
            &loss,
            &mut r
        )
        .is_err()
    );
    o.retire_resource_catalog_visibility_v1(&p, &facts(), &mut r)
        .unwrap();
    let ResourceCatalogReadReplyV1::Page { .. } = page(&mut o, &c, 1, None, &mut r) else {
        panic!()
    };
    let mut changed = catalog::config(ResourceCatalogTableV1::Read, "a");
    changed.capabilities = ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read: true };
    mutate(&mut o, "a", changed, &mut r);
    let p = cut(&mut o, true, false, &mut r);
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
    mutate(
        &mut o,
        "a",
        catalog::config(ResourceCatalogTableV1::Read, "a"),
        &mut r,
    );
    let p = cut(&mut o, true, false, &mut r);
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
    o.retire_resource_catalog_visibility_v1(&p, &facts(), &mut r)
        .unwrap();
    assert!(matches!(
        page(&mut o, &sibling, 1, None, &mut r),
        ResourceCatalogReadReplyV1::Page { .. }
    ));
    o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
        .unwrap();
}
#[test]
fn selector_pressure_never_publishes_partial_aliases_and_cursors_are_limit_and_context_bound() {
    let mut l = limits();
    l.max_selectors = 1;
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    let (mut o, mut r) = setup(l);
    let c = acquire(&mut o, true, &mut r);
    let before = o.resource_catalog_read_usage_v1();
    let p = cut(&mut o, true, false, &mut r);
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
    assert_eq!(
        o.resource_catalog_read_usage_v1().issued_lifetime,
        before.issued_lifetime
    );
    assert_eq!(o.resource_catalog_read_usage_v1().selectors, 0);
    let (mut o, mut r) = setup(limits());
    let c = acquire(&mut o, true, &mut r);
    let sibling = acquire(&mut o, true, &mut r);
    let ResourceCatalogReadReplyV1::Page {
        entries,
        continuation,
        ..
    } = page(&mut o, &c, 1, None, &mut r)
    else {
        panic!()
    };
    for (id, query) in [
        (
            &c,
            ResourceCatalogReadQueryV1::List {
                limit: 2,
                continuation: continuation.as_ref(),
            },
        ),
        (
            &sibling,
            ResourceCatalogReadQueryV1::List {
                limit: 1,
                continuation: continuation.as_ref(),
            },
        ),
        (
            &sibling,
            ResourceCatalogReadQueryV1::Inspect {
                entry: &entries[0].reference,
            },
        ),
    ] {
        let p = cut(&mut o, true, false, &mut r);
        let before = o.resource_catalog_read_usage_v1().issued_lifetime;
        assert!(
            o.query_resource_catalog_read_v1(id, &scope(), &p, query, &facts(), &mut r)
                .is_err()
        );
        assert_eq!(o.resource_catalog_read_usage_v1().issued_lifetime, before);
    }
}
#[test]
fn exact_parent_capacity_and_early_or_late_faults_preserve_owned_obligations_and_issuance() {
    let l = limits();
    let mut o = catalog::bare();
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&[], catalog::limits(), &catalog::facts(), &mut r)
        .unwrap();
    o.configure_retention_budget(l.max_pool_bytes - 1, &mut r)
        .unwrap();
    assert!(matches!(
        o.configure_resource_catalog_reader_v1(&scope(), l, &facts(), &mut r),
        Err(ResourceError::Limit)
    ));
    assert_eq!(o.retention_usage().reserved, 0);
    for point in 0..2 {
        let (mut o, mut r) = setup(limits());
        let c = acquire(&mut o, true, &mut r);
        let sibling = acquire(&mut o, true, &mut r);
        let p = cut(&mut o, true, false, &mut r);
        let before = (
            o.generation(),
            o.retention_usage(),
            o.resource_catalog_read_usage_v1().issued_lifetime,
        );
        o.catalog_read_pool_mut().unwrap().query_fault = Some(point);
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
        assert_eq!((o.generation(), o.retention_usage()), (before.0, before.1));
        assert!(!o.faulted);
        assert_eq!(
            o.resource_catalog_read_usage_v1().issued_lifetime,
            before.2 + if point == 1 { 2 } else { 0 }
        );
        assert_eq!(o.resource_catalog_read_usage_v1().selectors, 0);
        assert!(matches!(
            page(&mut o, &sibling, 3, None, &mut r),
            ResourceCatalogReadReplyV1::Page { .. }
        ));
        o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
            .unwrap();
        o.stop_resource_catalog_read_v1(&sibling, &scope(), &facts(), &mut r)
            .unwrap();
        o.retire_resource_catalog_read_v1(&sibling, &facts(), &mut r)
            .unwrap();
        o.retire_resource_catalog_reader_v1(&facts(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn capture_modes_preserve_products_and_actual_inventory_funding_and_fault_cleanup() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    for mode in 0..4 {
        let (mut o, _) = setup(limits());
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let c = acquire(&mut o, true, &mut r);
        assert!(matches!(
            page(&mut o, &c, 3, None, &mut r),
            ResourceCatalogReadReplyV1::Page { .. }
        ));
        let inventory = inspect(&o, &mut Recorder::disabled());
        let view = inspect_read(&o, &inventory, &mut Recorder::disabled());
        assert!(view.coverage.all_owner_obligations);
        assert_eq!(view.funding.unrepresented_parent_bytes, 0);
        let row = view
            .owners
            .iter()
            .find(|row| row.owner.role() == ShutdownOwnerRoleV1::ResourceCatalogReads)
            .unwrap();
        assert_eq!(
            row.reserved_bytes,
            Some(o.resource_catalog_read_usage_v1().reserved_pool_bytes)
        );
        o.faulted = true;
        o.stop_resource_catalog_read_v1(&c, &scope(), &facts(), &mut r)
            .unwrap();
        o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
            .unwrap();
        o.retire_resource_catalog_reader_v1(&facts(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(o.faulted);
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
        if mode == 1 || mode == 2 {
            while let Some(record) = r.pop_front() {
                assert_eq!(record.service_time.is_some(), mode == 2);
            }
        }
    }
}

#[test]
fn visibility_requires_exact_tuple_coverage_and_contiguous_approved_order_without_side_effects() {
    let (mut o, mut r) = setup(limits());
    let root = o.resources.catalog.current().unwrap().0.clone();
    let foreign = catalog::address(ResourceCatalogTableV1::Read, "foreign");
    for variant in 0..8 {
        let mut rows: Vec<_> = root
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| ResourceCatalogVisibilityV1 {
                address: &row.address,
                visible: true,
                descriptor: true,
                order: Some(i),
            })
            .collect();
        match variant {
            0 => {
                rows.pop();
            }
            1 => rows[1].address = rows[0].address,
            2 => rows[1].order = Some(99),
            3 => rows[1].order = rows[0].order,
            4 => rows[1].visible = false,
            5 => rows[1].order = None,
            6 => {
                rows[1].visible = false;
                rows[1].descriptor = false;
            }
            _ => rows[1].address = &foreign,
        }
        let before = (
            o.generation(),
            o.retention_usage(),
            o.resource_catalog_read_usage_v1(),
        );
        assert!(
            o.prepare_resource_catalog_visibility_v1(&scope(), &rows, &facts(), &mut r)
                .is_err()
        );
        assert_eq!(
            (
                o.generation(),
                o.retention_usage(),
                o.resource_catalog_read_usage_v1()
            ),
            before
        );
    }
}

#[test]
fn cuts_follow_the_actual_engine_root_not_only_the_catalog_pointer_and_do_not_refresh() {
    let (mut o, mut r) = setup(limits());
    let c = acquire(&mut o, true, &mut r);
    let stale = cut(&mut o, true, false, &mut r);
    o.create(b"unrelated prose", &mut r).unwrap();
    assert!(
        o.query_resource_catalog_read_v1(
            &c,
            &scope(),
            &stale,
            ResourceCatalogReadQueryV1::List {
                limit: 1,
                continuation: None
            },
            &facts(),
            &mut r
        )
        .is_err()
    );
    assert!(matches!(
        page(&mut o, &c, 3, None, &mut r),
        ResourceCatalogReadReplyV1::Page { .. }
    ));
    o.retire_resource_catalog_visibility_v1(&stale, &facts(), &mut r)
        .unwrap();
    let p = cut(&mut o, true, false, &mut r);
    // Reused preparation slots have fresh private markers, never ABA authority.
    assert!(
        o.query_resource_catalog_read_v1(
            &c,
            &scope(),
            &stale,
            ResourceCatalogReadQueryV1::List {
                limit: 1,
                continuation: None
            },
            &facts(),
            &mut r
        )
        .is_err()
    );
    o.retire_resource_catalog_visibility_v1(&p, &facts(), &mut r)
        .unwrap();
}

#[test]
fn all_current_release_gates_distinguish_absorbing_loss_from_unknown_or_transient_refusal() {
    for field in 0..9 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = setup(limits());
            let c = acquire(&mut o, true, &mut r);
            let p = cut(&mut o, true, false, &mut r);
            let mut f = facts();
            *match field {
                0 => &mut f.authorized,
                1 => &mut f.scope,
                2 => &mut f.continuous,
                3 => &mut f.live,
                4 => &mut f.retained,
                5 => &mut f.not_stopped,
                6 => &mut f.resources_ready,
                7 => &mut f.transfer_ready,
                _ => &mut f.reply_ready,
            } = value;
            assert!(
                o.query_resource_catalog_read_v1(
                    &c,
                    &scope(),
                    &p,
                    ResourceCatalogReadQueryV1::List {
                        limit: 1,
                        continuation: None
                    },
                    &f,
                    &mut r
                )
                .is_err()
            );
            let absorbing = field < 5 && value == Some(false);
            assert_eq!(
                o.catalog_read_pool().unwrap().contexts[c.slot]
                    .as_ref()
                    .unwrap()
                    .invalidated,
                absorbing
            );
            assert_eq!(
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
                .is_ok(),
                !absorbing
            );
        }
    }
}
