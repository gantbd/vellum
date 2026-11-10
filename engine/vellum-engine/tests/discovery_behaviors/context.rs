//! One owner and issuance domain, not one registry per descriptive profile.
use super::*;
use vellum_engine::capture::CapturedValueV1 as C;

fn scoped(profile: &str) -> DiscoveryScopeV1 {
    let mut c = scope();
    c.context.profile = profile.into();
    c
}
fn visible<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(order),
        schema: true,
        presentation: false,
        advice: false,
        variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
            order: Some(i),
            enabled: None,
        }),
    }
}
pub(super) fn visibility() -> NativeBuiltinDiscoveryVisibilityV2 {
    NativeBuiltinDiscoveryVisibilityV2 {
        legacy: NativeBuiltinDiscoveryVisibilityV1 {
            annotation: visible(0),
            history: visible(1),
        },
        context: visible(2),
    }
}
pub(super) fn profile_limits() -> DiscoveryLimitsV1 {
    DiscoveryLimitsV1 {
        max_row_bytes: 262144,
        max_packet_bytes: 1048576,
        max_capture_bytes: 1048576,
        max_reply_bytes: 262144,
        max_held_bytes: 16777216,
        metadata: CaptureLimitsV1 {
            max_bytes: 262144,
            max_nodes: 4096,
            max_depth: 16,
        },
        ..limits()
    }
}
pub(super) fn setup_with(l: DiscoveryLimitsV1) -> support::Fixture {
    let mut f = support::Fixture::new();
    f.owner
        .configure_retention_budget(33554432, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_discovery_v1(&scope(), l, &facts(), &mut f.recorder)
        .unwrap();
    f
}
fn prepare(
    f: &mut support::Fixture,
    c: &DiscoveryScopeV1,
    v: &NativeBuiltinDiscoveryVisibilityV2,
) -> DiscoveryCutIdV1 {
    if c.context.profile == OWNED_CONTRACT_DISCOVERY_PROFILE_V2 {
        f.owner
            .prepare_native_builtin_discovery_cut_v2(c, v, None, false, &facts(), &mut f.recorder)
            .unwrap()
    } else {
        f.owner
            .prepare_native_builtin_discovery_cut_v1(
                c,
                &v.legacy,
                None,
                false,
                &facts(),
                &mut f.recorder,
            )
            .unwrap()
    }
}
fn query(
    f: &mut support::Fixture,
    id: &DiscoveryContextIdV1,
    c: &DiscoveryScopeV1,
    v: &NativeBuiltinDiscoveryVisibilityV2,
    q: DiscoveryQueryV1<'_>,
) -> Result<DiscoveryReplyV1, DiscoveryErrorV1> {
    let p = prepare(f, c, v);
    f.owner
        .query_discovery_v1(id, c, &p, q, &facts(), &mut f.recorder)
}
fn field<'a>(c: &'a vellum_engine::capture::InertCaptureV1, v: &'a C, name: &str) -> &'a C {
    let C::Object(rows) = v else { panic!() };
    c.node(rows.iter().find(|(k, _)| k == name).unwrap().1)
        .unwrap()
}

#[test]
fn schema_profile_lists_three_fixed_rows_and_releases_two_ten_form_context_variants() {
    let mut f = setup_with(profile_limits());
    let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
    let v = visibility();
    let generation = f.owner.generation();
    let old = f
        .owner
        .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder)
        .unwrap();
    let rows = f
        .owner
        .native_builtin_discovery_rows_v2(&c, &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(rows[0].command, old[0].command);
    assert_eq!(rows[1].command, old[1].command);
    assert_eq!(rows[2].contract, "engine-context-declaration/1");
    let p = prepare(&mut f, &c, &v);
    let id = f
        .owner
        .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let DiscoveryReplyV1::Page { entries, .. } = query(
        &mut f,
        &id,
        &c,
        &v,
        DiscoveryQueryV1::List {
            limit: 3,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(entries.len(), 3);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = query(
        &mut f,
        &id,
        &c,
        &v,
        DiscoveryQueryV1::Inspect {
            entry: &entries[2].reference,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(descriptor.variants.len(), 2);
    for (variant, expected) in descriptor.variants.iter().zip([6, 4]) {
        let schema = variant.schema.as_ref().unwrap();
        let C::Array(branches) = field(
            schema,
            field(schema, field(schema, schema.root(), "input"), "schema"),
            "oneOf",
        ) else {
            panic!()
        };
        assert_eq!(branches.len(), expected);
    }
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.discovery_context_usage_v1(&id).unwrap().entries, 3);
}

#[test]
fn schema_and_legacy_contexts_share_capacity_and_spent_issuance_even_after_retirement() {
    for lifetime in [false, true] {
        let mut l = profile_limits();
        if lifetime {
            l.max_issued = 1;
        } else {
            l.max_contexts = 1;
        }
        let mut f = setup_with(l);
        let v = visibility();
        let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
        let charge = f.owner.retention_usage();
        let p = prepare(&mut f, &c, &v);
        let id = f
            .owner
            .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
            .unwrap()
            .context;
        f.owner
            .configure_discovery_v1(&c, l, &facts(), &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.retention_usage(), charge);
        let old = prepare(&mut f, &scope(), &v);
        assert!(matches!(
            f.owner
                .acquire_discovery_v1(&scope(), &old, &facts(), &mut f.recorder),
            Err(DiscoveryErrorV1::Limit)
        ));
        f.owner
            .stop_discovery_context_v1(&id, &c, &facts(), &mut f.recorder)
            .unwrap();
        f.owner
            .retire_discovery_context_v1(&id, &facts(), &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.discovery_usage_v1().unwrap().issued_lifetime, 1);
        let old = prepare(&mut f, &scope(), &v);
        let result = f
            .owner
            .acquire_discovery_v1(&scope(), &old, &facts(), &mut f.recorder);
        assert_eq!(result.is_ok(), !lifetime);
    }
}

#[test]
fn wrong_profile_cannot_consume_a_cut_apply_loss_or_substitute_a_source() {
    let mut f = setup_with(profile_limits());
    let v = visibility();
    let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
    let p = prepare(&mut f, &c, &v);
    let id = f
        .owner
        .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let p = prepare(&mut f, &c, &v);
    let before = f.owner.discovery_usage_v1().unwrap();
    let mut lost = facts();
    lost.authorized = Some(false);
    assert!(
        f.owner
            .query_discovery_v1(
                &id,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 3,
                    continuation: None
                },
                &lost,
                &mut f.recorder
            )
            .is_err()
    );
    assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
    assert!(!f.owner.discovery_context_usage_v1(&id).unwrap().stopped);
    assert!(
        f.owner
            .query_discovery_v1(
                &id,
                &c,
                &p,
                DiscoveryQueryV1::List {
                    limit: 3,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
    assert!(
        f.owner
            .prepare_native_builtin_discovery_cut_v1(
                &c,
                &v.legacy,
                None,
                false,
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert!(
        f.owner
            .prepare_native_builtin_discovery_cut_v2(
                &scope(),
                &v,
                None,
                false,
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
}

#[test]
fn schema_disclosure_is_captured_current_and_never_an_enablement_claim() {
    let mut f = setup_with(profile_limits());
    let mut v = visibility();
    let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
    v.context.schema = false;
    let p = prepare(&mut f, &c, &v);
    let id = f
        .owner
        .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    v.context.schema = true;
    let DiscoveryReplyV1::Page { entries, .. } = query(
        &mut f,
        &id,
        &c,
        &v,
        DiscoveryQueryV1::List {
            limit: 3,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!()
    };
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = query(
        &mut f,
        &id,
        &c,
        &v,
        DiscoveryQueryV1::Inspect {
            entry: &entries[2].reference,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(descriptor.variants.iter().all(|v| v.schema.is_none()));
    assert!(
        !f.owner
            .discovery_context_usage_v1(&id)
            .unwrap()
            .target_captured
    );
}

#[test]
fn schema_source_refuses_partial_catalogs_and_independent_metadata_capacity_failures() {
    for case in 0..7 {
        let mut l = profile_limits();
        match case {
            0 => l.max_entries = 2,
            1 => l.max_variants = 3,
            2 => l.metadata.max_bytes = 32,
            3 => l.metadata.max_nodes = 16,
            4 => l.metadata.max_depth = 3,
            5 => l.max_row_bytes = 32,
            _ => l.max_packet_bytes = 32,
        }
        let mut f = setup_with(l);
        let before = f.owner.discovery_usage_v1().unwrap();
        assert!(
            f.owner
                .prepare_native_builtin_discovery_cut_v2(
                    &scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2),
                    &visibility(),
                    None,
                    false,
                    &facts(),
                    &mut f.recorder
                )
                .is_err(),
            "case {case}"
        );
        assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
    }
}

#[test]
fn schema_source_establishes_only_its_two_source_premises_not_visibility_authority_or_readiness() {
    for case in 0..7 {
        let mut f = setup_with(profile_limits());
        let mut g = facts();
        match case {
            0 => {
                g.accepted_registry_attested = Some(false);
                g.schema_descriptions_attested = None;
            }
            1 => g.authorized = Some(false),
            2 => g.visibility_attested = None,
            3 => g.visibility_complete = Some(false),
            4 => g.projection_order_attested = None,
            5 => g.resources_ready = Some(false),
            _ => g.interpretation_current = None,
        }
        let before = f.owner.discovery_usage_v1().unwrap();
        let result = f.owner.prepare_native_builtin_discovery_cut_v2(
            &scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2),
            &visibility(),
            None,
            false,
            &g,
            &mut f.recorder,
        );
        assert_eq!(result.is_ok(), case == 0);
        if case != 0 {
            assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
        }
    }
}

#[test]
fn narrowed_schema_stops_old_capture_but_unrelated_source_edits_only_stale_the_old_cut() {
    let mut f = setup_with(profile_limits());
    let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
    let mut v = visibility();
    let p = prepare(&mut f, &c, &v);
    let id = f
        .owner
        .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let stale = prepare(&mut f, &c, &v);
    f.owner.create(b"unrelated", &mut f.recorder).unwrap();
    assert!(
        f.owner
            .query_discovery_v1(
                &id,
                &c,
                &stale,
                DiscoveryQueryV1::List {
                    limit: 3,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert!(!f.owner.discovery_context_usage_v1(&id).unwrap().stopped);
    f.owner
        .retire_discovery_cut_v1(&stale, &facts(), &mut f.recorder)
        .unwrap();
    assert!(
        query(
            &mut f,
            &id,
            &c,
            &v,
            DiscoveryQueryV1::List {
                limit: 3,
                continuation: None
            }
        )
        .is_ok()
    );
    let spent = f.owner.discovery_usage_v1().unwrap().issued_lifetime;
    v.context.schema = false;
    assert!(
        query(
            &mut f,
            &id,
            &c,
            &v,
            DiscoveryQueryV1::List {
                limit: 3,
                continuation: None
            }
        )
        .is_err()
    );
    assert!(f.owner.discovery_context_usage_v1(&id).unwrap().stopped);
    assert_eq!(f.owner.discovery_usage_v1().unwrap().issued_lifetime, spent);
}

#[test]
fn schema_preparation_is_timed_content_free_and_identical_with_capture_off_or_saturated() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 256] {
        let mut f = setup_with(profile_limits());
        let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
        let mut v = visibility();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let p = prepare(&mut f, &c, &v);
        let id = f
            .owner
            .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
            .unwrap()
            .context;
        assert_eq!(f.owner.discovery_context_usage_v1(&id).unwrap().variants, 8);
        v.context.order = Some(99);
        assert!(
            f.owner
                .prepare_native_builtin_discovery_cut_v2(
                    &c,
                    &v,
                    None,
                    false,
                    &facts(),
                    &mut f.recorder
                )
                .is_err()
        );
        if capacity == 256 {
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front())
                .filter(|v| v.boundary == Boundary::DiscoveryBuiltinPreparation)
                .collect();
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].disposition, Disposition::Completed);
            assert_eq!(records[1].disposition, Disposition::Refused);
            assert!(records.iter().all(|v| v.service_time.is_some()));
            assert!(!format!("{records:?}").contains("engine-context"));
        } else if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn cross_profile_selectors_share_a_limit_and_cursors_remain_original_profile_bound() {
    let mut l = profile_limits();
    l.max_selectors = 4;
    l.max_page_rows = 2;
    let mut f = setup_with(l);
    let c = scoped(OWNED_CONTRACT_DISCOVERY_PROFILE_V2);
    let v = visibility();
    let p = prepare(&mut f, &c, &v);
    let id = f
        .owner
        .acquire_discovery_v1(&c, &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let DiscoveryReplyV1::Page {
        continuation: Some(cursor),
        ..
    } = query(
        &mut f,
        &id,
        &c,
        &v,
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: None,
        },
    )
    .unwrap()
    else {
        panic!()
    };
    let before = f.owner.discovery_usage_v1().unwrap();
    assert_eq!(before.selectors, 3);
    let old_cut = prepare(&mut f, &scope(), &v);
    assert!(
        f.owner
            .query_discovery_v1(
                &id,
                &scope(),
                &old_cut,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: Some(&cursor)
                },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    f.owner
        .retire_discovery_cut_v1(&old_cut, &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
    let DiscoveryReplyV1::Page {
        entries,
        continuation: None,
        ..
    } = query(
        &mut f,
        &id,
        &c,
        &v,
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: Some(&cursor),
        },
    )
    .unwrap()
    else {
        panic!()
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(f.owner.discovery_usage_v1().unwrap().selectors, 4);
    let p = prepare(&mut f, &scope(), &v);
    let old = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let spent = f.owner.discovery_usage_v1().unwrap().issued_lifetime;
    assert!(matches!(
        query(
            &mut f,
            &old,
            &scope(),
            &v,
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None
            }
        ),
        Err(DiscoveryErrorV1::Limit)
    ));
    assert_eq!(f.owner.discovery_usage_v1().unwrap().issued_lifetime, spent);
    f.owner
        .stop_discovery_context_v1(&id, &c, &facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_context_v1(&id, &facts(), &mut f.recorder)
        .unwrap();
    assert!(
        query(
            &mut f,
            &old,
            &scope(),
            &v,
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None
            }
        )
        .is_ok()
    );
    assert!(f.owner.discovery_usage_v1().unwrap().issued_lifetime > spent);
}
