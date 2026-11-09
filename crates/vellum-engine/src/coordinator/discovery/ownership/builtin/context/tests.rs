use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
#[test]
fn installed_context_selectors_require_private_origin_and_the_exact_operation_family() {
    for builtin in [false, true] {
        let (mut o, mut r) = setup();
        let p = prepare(&mut o, builtin, &mut r);
        let c = o
            .acquire_discovery_v1(&scope2(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &c,
                &scope2(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 3,
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
                &scope2(),
                &p,
                DiscoveryQueryV1::Inspect {
                    entry: &entries[2].reference,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let context = o.discovery_context(&c).unwrap();
        for (index, operation) in ["semantic-settings", "supported-registry"]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                queries::context_selector_matches(
                    context,
                    &entries[2].reference,
                    &descriptor.variants[index].reference,
                    operation
                )
                .is_ok(),
                builtin
            );
            assert!(
                queries::context_selector_matches(
                    context,
                    &entries[2].reference,
                    &descriptor.variants[1 - index].reference,
                    operation
                )
                .is_err()
            );
            assert!(
                queries::context_selector_matches(
                    context,
                    &entries[0].reference,
                    &descriptor.variants[index].reference,
                    operation
                )
                .is_err()
            );
        }
        assert!(
            queries::context_selector_matches(
                context,
                &entries[2].reference,
                &descriptor.variants[0].reference,
                "surprise"
            )
            .is_err()
        );
    }
}
fn scope2() -> DiscoveryScopeV1 {
    let mut c = scope();
    c.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V2.into();
    c
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV2 {
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
    NativeBuiltinDiscoveryVisibilityV2 {
        legacy: NativeBuiltinDiscoveryVisibilityV1 {
            annotation: show(0),
            history: show(1),
        },
        context: show(2),
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
    l.max_page_rows = 3;
    l.max_held_bytes = 8388608;
    l.metadata = crate::capture::CaptureLimitsV1 {
        max_bytes: 262144,
        max_nodes: 4096,
        max_depth: 16,
    };
    o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
        .unwrap();
    (o, r)
}
fn prepare(o: &mut SourceCoordinator, builtin: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if builtin {
        return o
            .prepare_native_builtin_discovery_cut_v2(
                &scope2(),
                &visibility(),
                None,
                false,
                &facts(),
                r,
            )
            .unwrap();
    }
    let rows = o
        .native_builtin_discovery_rows_v2(&scope2(), &facts(), r)
        .unwrap();
    let commands = [rows[0].command.clone(), rows[1].command.clone()];
    with_inputs_v2(
        &rows[0].authority,
        &commands,
        &rows[2].command,
        &visibility(),
        None,
        false,
        |input| o.prepare_discovery_cut_v1(&scope2(), input, &facts(), r),
    )
    .unwrap()
    .unwrap()
}
#[test]
fn schema_generic_and_builtin_identical_metadata_cannot_replace_each_other_or_apply_loss() {
    for builtin in [false, true] {
        for loss in [false, true] {
            let (mut o, mut r) = setup();
            let p = prepare(&mut o, builtin, &mut r);
            let id = o
                .acquire_discovery_v1(&scope2(), &p, &facts(), &mut r)
                .unwrap()
                .context;
            let wrong = prepare(&mut o, !builtin, &mut r);
            let before = o.discovery_usage_v1().unwrap();
            let mut g = facts();
            if loss {
                g.authorized = Some(false);
            }
            assert!(
                o.query_discovery_v1(
                    &id,
                    &scope2(),
                    &wrong,
                    DiscoveryQueryV1::List {
                        limit: 3,
                        continuation: None
                    },
                    &g,
                    &mut r
                )
                .is_err()
            );
            assert_eq!(o.discovery_usage_v1().unwrap(), before);
            assert!(!o.discovery_context_usage_v1(&id).unwrap().stopped);
            o.retire_discovery_cut_v1(&wrong, &facts(), &mut r).unwrap();
            let correct = prepare(&mut o, builtin, &mut r);
            assert!(
                o.query_discovery_v1(
                    &id,
                    &scope2(),
                    &correct,
                    DiscoveryQueryV1::List {
                        limit: 3,
                        continuation: None
                    },
                    &facts(),
                    &mut r
                )
                .is_ok()
            );
        }
    }
}

#[test]
fn owned_source_profile_corruption_is_an_integrity_fault_not_another_valid_profile() {
    let (mut o, mut r) = setup();
    let p = prepare(&mut o, true, &mut r);
    o.discovery_store_mut().unwrap().packets[p.slot]
        .as_mut()
        .unwrap()
        .scope
        .context
        .profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    assert_eq!(o.discovery_usage_v1(), Err(Error::Faulted));
}
