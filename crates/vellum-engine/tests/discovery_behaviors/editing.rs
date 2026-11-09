use super::*;
use vellum_engine::capture::CapturedValueV1 as C;
fn scope3() -> DiscoveryScopeV1 {
    let mut c = scope();
    c.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V3.into();
    c
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV3 {
    NativeBuiltinDiscoveryVisibilityV3 {
        previous: context::visibility(),
        ordinary: NativeBuiltinEntryVisibilityV1 {
            order: Some(3),
            schema: true,
            presentation: false,
            advice: false,
            variants: [NativeBuiltinVariantVisibilityV1 {
                order: Some(0),
                enabled: None,
            }],
        },
    }
}
fn prepare(f: &mut support::Fixture, v: &NativeBuiltinDiscoveryVisibilityV3) -> DiscoveryCutIdV1 {
    f.owner
        .prepare_native_builtin_discovery_cut_v3(
            &scope3(),
            v,
            None,
            false,
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
}
fn query(
    f: &mut support::Fixture,
    id: &DiscoveryContextIdV1,
    v: &NativeBuiltinDiscoveryVisibilityV3,
    q: DiscoveryQueryV1<'_>,
) -> Result<DiscoveryReplyV1, DiscoveryErrorV1> {
    let p = prepare(f, v);
    f.owner
        .query_discovery_v1(id, &scope3(), &p, q, &facts(), &mut f.recorder)
}
fn field<'a>(c: &'a vellum_engine::capture::InertCaptureV1, v: &'a C, key: &str) -> &'a C {
    let C::Object(rows) = v else { panic!() };
    c.node(rows.iter().find(|(k, _)| k == key).unwrap().1)
        .unwrap()
}
fn text(v: &C) -> &str {
    let C::Text(v) = v else { panic!() };
    v
}
#[test]
fn editing_profile_preserves_prefix_and_releases_exact_four_handler_bound_forms() {
    let mut f = context::setup_with(context::profile_limits());
    let v = visibility();
    let generation = f.owner.generation();
    let mut c2 = scope();
    c2.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V2.into();
    let old = f
        .owner
        .native_builtin_discovery_rows_v2(&c2, &facts(), &mut f.recorder)
        .unwrap();
    let rows = f
        .owner
        .native_builtin_discovery_rows_v3(&scope3(), &facts(), &mut f.recorder)
        .unwrap();
    for i in 0..3 {
        assert_eq!(rows[i].command, old[i].command);
        assert_eq!(rows[i].contract, old[i].contract);
    }
    assert_eq!(rows[3].contract, "engine-ordinary-edit-declaration/1");
    let p = prepare(&mut f, &v);
    let id = f
        .owner
        .acquire_discovery_v1(&scope3(), &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let DiscoveryReplyV1::Page { entries, .. } = query(
        &mut f,
        &id,
        &v,
        DiscoveryQueryV1::List {
            limit: 4,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(entries.len(), 4);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = query(
        &mut f,
        &id,
        &v,
        DiscoveryQueryV1::Inspect {
            entry: &entries[3].reference,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(descriptor.variants.len(), 1);
    let schema = descriptor.variants[0].schema.as_ref().unwrap();
    let input = field(schema, schema.root(), "input");
    assert_eq!(
        text(field(schema, input, "coverage")),
        "outer-structure-only"
    );
    let C::Array(branches) = field(schema, field(schema, input, "schema"), "oneOf") else {
        panic!()
    };
    assert_eq!(branches.len(), 4);
    for (index, (contract, profile, count)) in [
        ("selection-edit/1", "explicit-positional-selection/1", 10),
        ("line-edit/1", "explicit-prefix-line-edit/1", 10),
        ("boundary-command/1", "unicode-logical-edit/1", 13),
        ("configured-line-edit/1", "configured-prefix-line/1", 10),
    ]
    .into_iter()
    .enumerate()
    {
        let b = schema.node(branches[index]).unwrap();
        let props = field(schema, b, "properties");
        assert_eq!(
            text(field(schema, field(schema, props, "contract"), "const")),
            contract
        );
        assert_eq!(
            text(field(schema, field(schema, props, "profile"), "const")),
            profile
        );
        let C::Array(required) = field(schema, b, "required") else {
            panic!()
        };
        assert_eq!(required.len(), count);
        assert!(matches!(
            field(schema, b, "additionalProperties"),
            C::Boolean(false)
        ));
    }
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn editing_profile_cannot_rearm_shared_context_or_lifetime_capacity() {
    for lifetime in [false, true] {
        let mut l = context::profile_limits();
        if lifetime {
            l.max_issued = 1;
        } else {
            l.max_contexts = 1;
        }
        let mut f = context::setup_with(l);
        let v = visibility();
        let p = prepare(&mut f, &v);
        let id = f
            .owner
            .acquire_discovery_v1(&scope3(), &p, &facts(), &mut f.recorder)
            .unwrap()
            .context;
        f.owner
            .configure_discovery_v1(&scope3(), l, &facts(), &mut f.recorder)
            .unwrap();
        let old = f
            .owner
            .prepare_native_builtin_discovery_cut_v1(
                &scope(),
                &v.previous.legacy,
                None,
                false,
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        assert!(
            f.owner
                .acquire_discovery_v1(&scope(), &old, &facts(), &mut f.recorder)
                .is_err()
        );
        f.owner
            .stop_discovery_context_v1(&id, &scope3(), &facts(), &mut f.recorder)
            .unwrap();
        f.owner
            .retire_discovery_context_v1(&id, &facts(), &mut f.recorder)
            .unwrap();
        let old = f
            .owner
            .prepare_native_builtin_discovery_cut_v1(
                &scope(),
                &v.previous.legacy,
                None,
                false,
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(
            f.owner
                .acquire_discovery_v1(&scope(), &old, &facts(), &mut f.recorder)
                .is_ok(),
            !lifetime
        );
    }
}

#[test]
fn editing_profile_wrong_scope_cannot_consume_or_poison_and_disclosure_cannot_widen() {
    for capture_schema in [false, true] {
        let mut f = context::setup_with(context::profile_limits());
        let mut v = visibility();
        v.ordinary.schema = capture_schema;
        let p = prepare(&mut f, &v);
        let id = f
            .owner
            .acquire_discovery_v1(&scope3(), &p, &facts(), &mut f.recorder)
            .unwrap()
            .context;
        let p = prepare(&mut f, &v);
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
                        limit: 4,
                        continuation: None
                    },
                    &lost,
                    &mut f.recorder
                )
                .is_err()
        );
        assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
        let DiscoveryReplyV1::Page { entries, .. } = f
            .owner
            .query_discovery_v1(
                &id,
                &scope3(),
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
            panic!()
        };
        v.ordinary.schema = !capture_schema;
        let out = query(
            &mut f,
            &id,
            &v,
            DiscoveryQueryV1::Inspect {
                entry: &entries[3].reference,
            },
        );
        if capture_schema {
            assert!(out.is_err());
            assert!(f.owner.discovery_context_usage_v1(&id).unwrap().stopped);
        } else {
            let DiscoveryReplyV1::Descriptor { descriptor, .. } = out.unwrap() else {
                panic!()
            };
            assert!(descriptor.variants.iter().all(|v| v.schema.is_none()));
        }
    }
}

#[test]
fn editing_source_never_installs_a_partial_cut_when_complete_capacity_is_missing() {
    for case in 0..7 {
        let mut l = context::profile_limits();
        match case {
            0 => l.max_entries = 3,
            1 => l.max_variants = 3,
            2 => l.metadata.max_bytes = 32,
            3 => l.metadata.max_nodes = 16,
            4 => l.metadata.max_depth = 3,
            5 => l.max_row_bytes = 32,
            _ => l.max_packet_bytes = 32,
        }
        let mut f = context::setup_with(l);
        let before = f.owner.discovery_usage_v1().unwrap();
        assert!(
            f.owner
                .prepare_native_builtin_discovery_cut_v3(
                    &scope3(),
                    &visibility(),
                    None,
                    false,
                    &facts(),
                    &mut f.recorder
                )
                .is_err()
        );
        assert_eq!(f.owner.discovery_usage_v1().unwrap(), before);
    }
}

#[test]
fn editing_preparation_is_observed_and_preserves_state_with_capture_off_or_saturated() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 256] {
        let mut f = context::setup_with(context::profile_limits());
        let mut v = visibility();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let before = f.owner.generation();
        let p = prepare(&mut f, &v);
        let id = f
            .owner
            .acquire_discovery_v1(&scope3(), &p, &facts(), &mut f.recorder)
            .unwrap()
            .context;
        assert_eq!(f.owner.discovery_context_usage_v1(&id).unwrap().variants, 9);
        v.ordinary.order = Some(99);
        assert!(
            f.owner
                .prepare_native_builtin_discovery_cut_v3(
                    &scope3(),
                    &v,
                    None,
                    false,
                    &facts(),
                    &mut f.recorder
                )
                .is_err()
        );
        assert_eq!(f.owner.generation(), before);
        if capacity == 256 {
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front())
                .filter(|v| v.boundary == Boundary::DiscoveryBuiltinPreparation)
                .collect();
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].disposition, Disposition::Completed);
            assert_eq!(records[1].disposition, Disposition::Refused);
            assert!(records.iter().all(|v| v.service_time.is_some()));
            assert!(!format!("{records:?}").contains("ordinary-edit"));
        } else if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
    }
}
