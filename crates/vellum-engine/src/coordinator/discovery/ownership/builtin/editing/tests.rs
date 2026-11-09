use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
#[test]
fn ordinary_dispatch_recognizes_only_original_installed_handler_selectors() {
    for builtin in [false, true] {
        let (mut o, mut r) = setup();
        let p = prepare(&mut o, builtin, &mut r);
        let c = o
            .acquire_discovery_v1(&scope3(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &c,
                &scope3(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 4,
                    continuation: None,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &c,
                &scope3(),
                &p,
                DiscoveryQueryV1::Inspect {
                    entry: &entries[3].reference,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let context = o.discovery_context(&c).unwrap();
        assert_eq!(
            queries::ordinary_selector_matches(
                context,
                &entries[3].reference,
                &descriptor.variants[0].reference
            )
            .is_ok(),
            builtin
        );
        assert!(
            queries::ordinary_selector_matches(
                context,
                &entries[0].reference,
                &descriptor.variants[0].reference
            )
            .is_err()
        );
    }
}
fn scope3() -> DiscoveryScopeV1 {
    let mut c = scope();
    c.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V3.into();
    c
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV3 {
    fn show<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
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
    NativeBuiltinDiscoveryVisibilityV3 {
        previous: NativeBuiltinDiscoveryVisibilityV2 {
            legacy: NativeBuiltinDiscoveryVisibilityV1 {
                annotation: show(0),
                history: show(1),
            },
            context: show(2),
        },
        ordinary: show(3),
    }
}
fn setup() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
    o.configure_retention_budget(16777216, &mut r).unwrap();
    let mut l = limits();
    l.max_row_bytes = 262144;
    l.max_packet_bytes = 1048576;
    l.max_capture_bytes = 1048576;
    l.max_reply_bytes = 262144;
    l.max_page_rows = 4;
    l.max_held_bytes = 8388608;
    l.metadata = CaptureLimitsV1 {
        max_bytes: 262144,
        max_nodes: 4096,
        max_depth: 16,
    };
    o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
        .unwrap();
    (o, r)
}
fn generic<T>(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    consume: impl FnOnce(&mut SourceCoordinator, DiscoveryCutInputV1<'_>, &mut Recorder) -> T,
) -> T {
    let rows = o
        .native_builtin_discovery_rows_v3(&scope3(), &facts(), r)
        .unwrap();
    let commands = std::array::from_fn(|i| rows[i].command.clone());
    with_inputs_v3(
        &rows[0].authority,
        &commands,
        &visibility(),
        None,
        false,
        |input| consume(o, input, r),
    )
    .unwrap()
}
fn prepare(o: &mut SourceCoordinator, builtin: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if builtin {
        o.prepare_native_builtin_discovery_cut_v3(
            &scope3(),
            &visibility(),
            None,
            false,
            &facts(),
            r,
        )
        .unwrap()
    } else {
        generic(o, r, |o, input, r| {
            o.prepare_discovery_cut_v1(&scope3(), input, &facts(), r)
        })
        .unwrap()
    }
}
#[test]
fn editing_generic_lookalikes_cannot_replace_private_origin_or_apply_loss() {
    for builtin in [false, true] {
        let (mut o, mut r) = setup();
        let p = prepare(&mut o, builtin, &mut r);
        let id = o
            .acquire_discovery_v1(&scope3(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let wrong = prepare(&mut o, !builtin, &mut r);
        let before = o.discovery_usage_v1().unwrap();
        let mut lost = facts();
        lost.authorized = Some(false);
        assert!(
            o.query_discovery_v1(
                &id,
                &scope3(),
                &wrong,
                DiscoveryQueryV1::List {
                    limit: 4,
                    continuation: None
                },
                &lost,
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.discovery_usage_v1().unwrap(), before);
        assert!(!o.discovery_context_usage_v1(&id).unwrap().stopped);
        o.retire_discovery_cut_v1(&wrong, &facts(), &mut r).unwrap();
        let p = prepare(&mut o, builtin, &mut r);
        assert!(
            o.query_discovery_v1(
                &id,
                &scope3(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 4,
                    continuation: None
                },
                &facts(),
                &mut r
            )
            .is_ok()
        );
    }
}
#[test]
fn actual_editing_ingestion_rejects_f16_schema_substitution_before_packet_installation() {
    let (mut o, mut r) = setup();
    let before = o.discovery_usage_v1().unwrap();
    let result = generic(&mut o, &mut r, |o, input, r| {
        let schema = property(input.entries[2].variants[0].input, "schema").unwrap();
        let forged = [
            ("contract", D::Text(ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1)),
            ("variant", D::Text("ordinary-edit")),
            ("coverage", D::Text("outer-structure-only")),
            ("schema", schema),
        ];
        let old = &input.entries[3].variants[0];
        let variants = [DiscoveryVariantInputV1 {
            id: old.id,
            input: D::Object(&forged),
            outcomes: old.outcomes,
            requirements: old.requirements,
            limits: old.limits,
            cancellation: old.cancellation,
            order: old.order,
            enabled: old.enabled,
        }];
        let mut rows: [DiscoveryEntryInputV1<'_>; 4] =
            std::array::from_fn(|i| entry(&input.entries[i]));
        rows[3].variants = &variants;
        o.prepare_discovery_cut_v1(
            &scope3(),
            DiscoveryCutInputV1 {
                entries: &rows,
                target: None,
                advice_present: false,
            },
            &facts(),
            r,
        )
    });
    assert!(matches!(result, Err(Error::InvalidRequest)));
    assert_eq!(o.discovery_usage_v1().unwrap(), before);
}
