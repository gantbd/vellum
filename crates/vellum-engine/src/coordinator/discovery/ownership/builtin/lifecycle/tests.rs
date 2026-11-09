use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
fn scope4() -> DiscoveryScopeV1 {
    let mut c = scope();
    c.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V4.into();
    c
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV4 {
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
    NativeBuiltinDiscoveryVisibilityV4 {
        previous: NativeBuiltinDiscoveryVisibilityV3 {
            previous: NativeBuiltinDiscoveryVisibilityV2 {
                legacy: NativeBuiltinDiscoveryVisibilityV1 {
                    annotation: show(0),
                    history: show(1),
                },
                context: show(2),
            },
            ordinary: show(3),
        },
        lifecycle: show(4),
    }
}
fn prepare(o: &mut SourceCoordinator, builtin: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if builtin {
        return o
            .prepare_native_builtin_discovery_cut_v4(
                &scope4(),
                &visibility(),
                None,
                false,
                &facts(),
                r,
            )
            .unwrap();
    }
    let rows = o
        .native_builtin_discovery_rows_v4(&scope4(), &facts(), r)
        .unwrap();
    let commands = std::array::from_fn(|i| rows[i].command.clone());
    editing::with_inputs_v3(
        &rows[0].authority,
        &commands,
        &visibility().previous,
        None,
        false,
        |prefix| {
            with_lifecycle_inputs(
                &rows[0].authority,
                &rows[4].command,
                &visibility(),
                prefix,
                |input| o.prepare_discovery_cut_v1(&scope4(), input, &facts(), r),
            )
        },
    )
    .unwrap()
    .unwrap()
    .unwrap()
}
#[test]
fn identical_generic_lifecycle_descriptors_never_install_an_executable_route() {
    for builtin in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        o.configure_retention_budget(16777216, &mut r).unwrap();
        let mut l = limits();
        l.max_entries = 8;
        l.max_page_rows = 5;
        l.max_selectors = 16;
        l.max_row_bytes = 262144;
        l.max_packet_bytes = 1048576;
        l.max_capture_bytes = 1048576;
        l.max_reply_bytes = 262144;
        l.max_held_bytes = 8388608;
        l.metadata = CaptureLimitsV1 {
            max_bytes: 262144,
            max_nodes: 4096,
            max_depth: 16,
        };
        o.configure_discovery_v1(&scope4(), l, &facts(), &mut r)
            .unwrap();
        let p = prepare(&mut o, builtin, &mut r);
        let c = o
            .acquire_discovery_v1(&scope4(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &c,
                &scope4(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 5,
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
                &scope4(),
                &p,
                DiscoveryQueryV1::Inspect {
                    entry: &entries[4].reference,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let context = o.discovery_context(&c).unwrap();
        for (operation, index) in [
            ("create-scratch", 0),
            ("attach-view", 1),
            ("detach-view", 1),
            ("membership", 2),
            ("dispose", 3),
        ] {
            assert_eq!(
                queries::lifecycle_selector_matches(
                    context,
                    &entries[4].reference,
                    &descriptor.variants[index].reference,
                    operation
                )
                .is_ok(),
                builtin
            );
            for wrong in 0..4 {
                if wrong != index {
                    assert!(
                        queries::lifecycle_selector_matches(
                            context,
                            &entries[4].reference,
                            &descriptor.variants[wrong].reference,
                            operation
                        )
                        .is_err()
                    );
                }
            }
        }
        assert!(
            queries::lifecycle_selector_matches(
                context,
                &entries[4].reference,
                &descriptor.variants[0].reference,
                "surprise"
            )
            .is_err()
        );
    }
}
