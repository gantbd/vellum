use super::*;
mod shutdown;
pub(super) fn limits() -> ResourceCatalogLimitsV1 {
    ResourceCatalogLimitsV1 {
        max_relations: 4,
        max_tag_bytes: 128,
        max_request_bytes: 8192,
        max_result_bytes: 8192,
        max_plan_bytes: 131072,
    }
}
pub(super) fn facts() -> TrustedResourceCatalogFactsV1 {
    TrustedResourceCatalogFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        registration_allowed: Some(true),
        identity_allowed: Some(true),
        registry_coverage_complete: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
    }
}
pub(super) fn bare() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 16,
            max_edit_payload_bytes: 4096,
        },
    })
}
pub(super) fn address(table: ResourceCatalogTableV1, id: &str) -> ResourceCatalogAddressV1 {
    ResourceCatalogAddressV1 {
        table,
        provider: ResourceCatalogIdentityV1 {
            authority: "a".into(),
            id: "p".into(),
            incarnation: 0,
        },
        resource: ResourceCatalogIdentityV1 {
            authority: "a".into(),
            id: id.into(),
            incarnation: 0,
        },
    }
}
pub(super) fn config(table: ResourceCatalogTableV1, key: &str) -> ResourceCatalogConfigurationV1 {
    ResourceCatalogConfigurationV1 {
        identity_domain: "d".into(),
        identity_key: key.into(),
        identity_known: false,
        locator: "PRIVATE_LOCATOR".into(),
        capabilities: match table {
            ResourceCatalogTableV1::Read => ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
                atomic_read: false,
                insertion_line_break: CatalogInsertionLineBreakV1::CrLf,
            },
            ResourceCatalogTableV1::Write => ResourceCatalogCapabilitiesV1::Write {
                procedure: "staged-replacement".into(),
                logical_continuity: false,
                synchronization_supported: false,
                conditional_write: false,
                causal_receipts: false,
                non_reuse_tokens: false,
                exclusive_create: false,
            },
        },
    }
}
fn install(
    table: ResourceCatalogTableV1,
    id: &str,
    expected: CatalogExpectationV1,
    key: &str,
) -> ResourceCatalogChangeV1 {
    ResourceCatalogChangeV1 {
        address: address(table, id),
        expected,
        action: ResourceCatalogActionV1::Install(config(table, key)),
    }
}
fn setup(initial: &[ResourceCatalogRowV1]) -> (SourceCoordinator, Recorder) {
    let mut o = bare();
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(initial, limits(), &facts(), &mut r)
        .unwrap();
    (o, r)
}
fn apply(
    o: &mut SourceCoordinator,
    changes: &[ResourceCatalogChangeV1],
    r: &mut Recorder,
) -> ResourceCatalogOutcomeV1 {
    let p = o.prepare_resource_catalog_v1(changes, &facts(), r).unwrap();
    o.commit_resource_catalog_v1(p, &facts(), r).unwrap()
}

#[test]
fn present_zero_is_not_absence_and_changed_installs_share_one_nonreused_owner_stamp() {
    let read = ResourceCatalogTableV1::Read;
    let write = ResourceCatalogTableV1::Write;
    let initial = [ResourceCatalogRowV1 {
        address: address(read, "r"),
        generation: 0,
        configuration: config(read, "old"),
    }];
    let (mut o, mut r) = setup(&initial);
    assert!(matches!(
        o.prepare_resource_catalog_v1(
            &[install(read, "r", CatalogExpectationV1::Absent, "new")],
            &facts(),
            &mut r
        ),
        Err(ResourceError::Stale)
    ));
    let before = o.generation();
    let out = apply(
        &mut o,
        &[
            install(
                read,
                "r",
                CatalogExpectationV1::Present { generation: 0 },
                "new",
            ),
            install(write, "w", CatalogExpectationV1::Absent, "new"),
        ],
        &mut r,
    );
    assert_eq!(out.status, ResourceCatalogStatusV1::Applied);
    assert_eq!(
        out.registrations
            .iter()
            .map(|v| v.generation)
            .collect::<Vec<_>>(),
        [Some(1), Some(1)]
    );
    assert_eq!(o.generation(), before + 1);
    let removals = [
        ResourceCatalogChangeV1 {
            address: address(read, "r"),
            expected: CatalogExpectationV1::Present { generation: 1 },
            action: ResourceCatalogActionV1::Remove,
        },
        ResourceCatalogChangeV1 {
            address: address(write, "w"),
            expected: CatalogExpectationV1::Present { generation: 1 },
            action: ResourceCatalogActionV1::Remove,
        },
    ];
    apply(&mut o, &removals, &mut r);
    assert_eq!(
        o.resource_catalog_usage_v1(&mut r)
            .unwrap()
            .registration_floor,
        1
    );
    assert!(
        apply(
            &mut o,
            &[install(read, "r", CatalogExpectationV1::Absent, "new")],
            &mut r
        )
        .registrations
        .iter()
        .all(|v| v.generation == Some(2))
    );
    assert!(
        !o.resource_catalog_registration_v1(&address(read, "r"), &mut r)
            .unwrap()
            .unwrap()
            .configuration
            .identity_known
    );
}

#[test]
fn noop_keeps_exact_root_at_publication_exhaustion_and_does_not_lower_the_floor() {
    let table = ResourceCatalogTableV1::Read;
    let initial = [ResourceCatalogRowV1 {
        address: address(table, "r"),
        generation: 9,
        configuration: config(table, "same"),
    }];
    let (mut o, mut r) = setup(&initial);
    o.generation = u64::MAX;
    let original = o.resources.catalog.root.as_ref().unwrap().clone();
    let out = apply(
        &mut o,
        &[install(
            table,
            "r",
            CatalogExpectationV1::Present { generation: 9 },
            "same",
        )],
        &mut r,
    );
    assert_eq!(out.status, ResourceCatalogStatusV1::Unchanged);
    assert!(Arc::ptr_eq(
        &original,
        o.resources.catalog.root.as_ref().unwrap()
    ));
    let p = o
        .prepare_resource_catalog_v1(
            &[install(
                table,
                "r",
                CatalogExpectationV1::Present { generation: 9 },
                "different",
            )],
            &facts(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.commit_resource_catalog_v1(p, &facts(), &mut r),
        Err(ResourceError::VersionExhausted)
    );
    assert!(Arc::ptr_eq(
        &original,
        o.resources.catalog.root.as_ref().unwrap()
    ));
    assert_eq!(o.resources.catalog.floor, 9);
}

#[test]
fn stamp_exhaustion_refuses_installs_but_not_removal_or_validated_absent_noops() {
    let table = ResourceCatalogTableV1::Read;
    let initial = [ResourceCatalogRowV1 {
        address: address(table, "r"),
        generation: MAX_STAMP,
        configuration: config(table, "same"),
    }];
    let (mut o, mut r) = setup(&initial);
    let removal = ResourceCatalogChangeV1 {
        address: address(table, "r"),
        expected: CatalogExpectationV1::Present {
            generation: MAX_STAMP,
        },
        action: ResourceCatalogActionV1::Remove,
    };
    apply(&mut o, &[removal], &mut r);
    assert_eq!(o.resources.catalog.floor, MAX_STAMP);
    assert!(matches!(
        o.prepare_resource_catalog_v1(
            &[install(table, "r", CatalogExpectationV1::Absent, "same")],
            &facts(),
            &mut r
        ),
        Err(ResourceError::VersionExhausted)
    ));
    let out = apply(
        &mut o,
        &[ResourceCatalogChangeV1 {
            address: address(table, "r"),
            expected: CatalogExpectationV1::Absent,
            action: ResourceCatalogActionV1::Remove,
        }],
        &mut r,
    );
    assert_eq!(out.status, ResourceCatalogStatusV1::Unchanged);
    assert_eq!(out.registrations[0].generation, None);
    assert_eq!(o.resources.catalog.floor, MAX_STAMP);
}

#[test]
fn stale_member_duplicate_address_and_wrong_table_configuration_never_publish_a_prefix() {
    let (mut o, mut r) = setup(&[]);
    let table = ResourceCatalogTableV1::Read;
    let generation = o.generation();
    let floor = o.resources.catalog.floor;
    for kind in 0..3 {
        let mut changes = vec![
            install(table, "a", CatalogExpectationV1::Absent, "a"),
            install(table, "b", CatalogExpectationV1::Absent, "b"),
        ];
        match kind {
            0 => changes[1].expected = CatalogExpectationV1::Present { generation: 0 },
            1 => changes[1].address = address(table, "a"),
            _ => {
                changes[1].action =
                    ResourceCatalogActionV1::Install(config(ResourceCatalogTableV1::Write, "b"))
            }
        }
        assert!(
            o.prepare_resource_catalog_v1(&changes, &facts(), &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.resources.catalog.floor, floor);
        assert!(
            o.resource_catalog_registration_v1(&address(table, "a"), &mut r)
                .unwrap()
                .is_none()
        );
    }
}

fn deny(g: &mut TrustedResourceCatalogFactsV1, field: usize, value: Option<bool>) {
    match field {
        0 => g.authorized = value,
        1 => g.scope = value,
        2 => g.continuous = value,
        3 => g.live = value,
        4 => g.not_stopped = value,
        5 => g.registration_allowed = value,
        6 => g.identity_allowed = value,
        7 => g.registry_coverage_complete = value,
        8 => g.companions_ready = value,
        9 => g.versions_available = value,
        10 => g.reply_ready = value,
        11 => g.resources_ready = value,
        12 => g.owner_control_allowed = value,
        _ => g.preparation_reserved = value,
    }
}
#[test]
fn every_native_fact_is_fresh_literal_true_at_both_preparation_and_commit() {
    for field in 0..14 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = setup(&[]);
            let changes = [install(
                ResourceCatalogTableV1::Read,
                "r",
                CatalogExpectationV1::Absent,
                "r",
            )];
            let mut g = facts();
            deny(&mut g, field, value);
            let generation = o.generation();
            assert!(matches!(
                o.prepare_resource_catalog_v1(&changes, &g, &mut r),
                Err(ResourceError::Unavailable)
            ));
            let p = o
                .prepare_resource_catalog_v1(&changes, &facts(), &mut r)
                .unwrap();
            assert_eq!(
                o.commit_resource_catalog_v1(p, &g, &mut r),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.resources.catalog.floor, 0);
        }
    }
}

#[test]
fn certificates_are_owner_and_current_root_bound_and_do_not_overwrite_unrelated_edits() {
    let (mut o, mut r) = setup(&[]);
    let changes = [install(
        ResourceCatalogTableV1::Read,
        "r",
        CatalogExpectationV1::Absent,
        "r",
    )];
    let p = o
        .prepare_resource_catalog_v1(&changes, &facts(), &mut r)
        .unwrap();
    let source = o.create(b"unrelated prose", &mut r).unwrap();
    let generation = o.generation();
    assert_eq!(
        o.commit_resource_catalog_v1(p, &facts(), &mut r),
        Err(ResourceError::Stale)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "unrelated prose"
    );
    let p = o
        .prepare_resource_catalog_v1(&changes, &facts(), &mut r)
        .unwrap();
    let (mut foreign, mut fr) = setup(&[]);
    assert_eq!(
        foreign.commit_resource_catalog_v1(p, &facts(), &mut fr),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(foreign.resources.catalog.floor, 0);
    assert!(!foreign.resources.catalog.faulted);
}

#[test]
fn corrupt_prediction_fences_catalog_without_fabricating_success_or_mutating_other_engine_state() {
    for drift in 0..4 {
        let (mut o, mut r) = setup(&[]);
        let changes = [install(
            ResourceCatalogTableV1::Read,
            "r",
            CatalogExpectationV1::Absent,
            "r",
        )];
        let mut p = o
            .prepare_resource_catalog_v1(&changes, &facts(), &mut r)
            .unwrap();
        match drift {
            0 => p.candidate.floor += 1,
            1 => p.candidate.outcome.registrations.clear(),
            2 => Arc::get_mut(&mut p.candidate.root).unwrap().rows.clear(),
            _ => p.request[0].expected = CatalogExpectationV1::Present { generation: 0 },
        }
        let generation = o.generation();
        assert_eq!(
            o.commit_resource_catalog_v1(p, &facts(), &mut r),
            Err(ResourceError::Faulted)
        );
        assert!(o.resources.catalog.faulted);
        assert_eq!(o.resources.catalog.floor, 0);
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
    }
}

#[test]
fn complete_native_request_result_plan_and_label_bounds_are_checked_before_any_clock_change() {
    let table = ResourceCatalogTableV1::Read;
    let changes = [install(table, "r", CatalogExpectationV1::Absent, "r")];
    let request = request_bytes(&changes, limits()).unwrap();
    let result = result_bytes(&changes).unwrap();
    let ResourceCatalogActionV1::Install(config) = &changes[0].action else {
        unreachable!()
    };
    let candidate = size_of::<Root>()
        + size_of::<ResourceCatalogRowV1>()
        + changes[0].address.payload().unwrap()
        + config.payload().unwrap();
    let plan = size_of::<PreparedResourceCatalogV1>() + request + candidate * 2 + result * 2;
    for field in 0..3 {
        for delta in [-1, 0, 1] {
            let mut l = limits();
            match field {
                0 => l.max_request_bytes = request.checked_add_signed(delta).unwrap(),
                1 => l.max_result_bytes = result.checked_add_signed(delta).unwrap(),
                _ => l.max_plan_bytes = plan.checked_add_signed(delta).unwrap(),
            }
            let mut o = bare();
            let mut r = Recorder::disabled();
            o.configure_resource_catalog_v1(&[], l, &facts(), &mut r)
                .unwrap();
            let generation = o.generation();
            assert_eq!(
                o.prepare_resource_catalog_v1(&changes, &facts(), &mut r)
                    .is_ok(),
                delta >= 0
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.resources.catalog.floor, 0);
        }
    }
    let mut l = limits();
    l.max_tag_bytes = "PRIVATE_LOCATOR".len() - 1;
    let mut o = bare();
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&[], l, &facts(), &mut r)
        .unwrap();
    assert!(matches!(
        o.prepare_resource_catalog_v1(&changes, &facts(), &mut r),
        Err(ResourceError::Malformed)
    ));
}

#[test]
fn final_catalog_capacity_not_batch_order_controls_replacement_and_failed_growth_has_no_prefix() {
    let table = ResourceCatalogTableV1::Read;
    let initial = [
        ResourceCatalogRowV1 {
            address: address(table, "a"),
            generation: 0,
            configuration: config(table, "a"),
        },
        ResourceCatalogRowV1 {
            address: address(table, "b"),
            generation: 0,
            configuration: config(table, "b"),
        },
    ];
    let mut l = limits();
    l.max_relations = 2;
    let mut o = bare();
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&initial, l, &facts(), &mut r)
        .unwrap();
    apply(
        &mut o,
        &[
            install(table, "c", CatalogExpectationV1::Absent, "c"),
            ResourceCatalogChangeV1 {
                address: address(table, "a"),
                expected: CatalogExpectationV1::Present { generation: 0 },
                action: ResourceCatalogActionV1::Remove,
            },
        ],
        &mut r,
    );
    assert_eq!(o.resource_catalog_usage_v1(&mut r).unwrap().read_rows, 2);
    assert_eq!(
        o.resource_catalog_registration_v1(&address(table, "b"), &mut r)
            .unwrap()
            .unwrap()
            .generation,
        0
    );
    let generation = o.generation();
    assert!(matches!(
        o.prepare_resource_catalog_v1(
            &[
                install(table, "d", CatalogExpectationV1::Absent, "d"),
                install(table, "e", CatalogExpectationV1::Absent, "e")
            ],
            &facts(),
            &mut r
        ),
        Err(ResourceError::Limit)
    ));
    assert_eq!(o.generation(), generation);
    assert_eq!(o.resources.catalog.floor, 1);
    assert!(
        o.resource_catalog_registration_v1(&address(table, "d"), &mut r)
            .unwrap()
            .is_none()
    );
}

#[test]
fn current_root_capacity_blocks_catalog_growth_and_exact_current_configuration_cannot_reset_the_clock()
 {
    let (mut o, mut r) = setup(&[]);
    let root = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(root, &mut r).unwrap();
    let changes = [install(
        ResourceCatalogTableV1::Read,
        "r",
        CatalogExpectationV1::Absent,
        "r",
    )];
    assert!(matches!(
        o.prepare_resource_catalog_v1(&changes, &facts(), &mut r),
        Err(ResourceError::Limit)
    ));
    assert_eq!(o.resources.catalog.floor, 0);
    assert_eq!(
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r),
        Ok(false)
    );
    assert_eq!(o.resources.catalog.floor, 0);
    let (mut o, mut r) = setup(&[]);
    apply(&mut o, &changes, &mut r);
    apply(
        &mut o,
        &[ResourceCatalogChangeV1 {
            address: address(ResourceCatalogTableV1::Read, "r"),
            expected: CatalogExpectationV1::Present { generation: 1 },
            action: ResourceCatalogActionV1::Remove,
        }],
        &mut r,
    );
    assert_eq!(
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r),
        Ok(false)
    );
    assert_eq!(o.resources.catalog.floor, 1);
    assert_eq!(
        apply(&mut o, &changes, &mut r).registrations[0].generation,
        Some(2)
    );
}
