use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
fn visibility<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
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
fn prepare(o: &mut SourceCoordinator, installed: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_selection_discovery_cut_v1(
                &scope(),
                &visibility(0),
                &visibility(1),
                &facts(),
                r,
            )
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let commands = [
        command(
            &store.engine,
            "F12-selection-v1",
            store.limits.max_context_bytes,
        )
        .unwrap(),
        command(
            &store.engine,
            "F12-boundary-movement-v1",
            store.limits.max_context_bytes,
        )
        .unwrap(),
    ];
    with_input(
        &authority,
        &commands,
        &visibility(0),
        &visibility(1),
        |input| o.prepare_discovery_cut_v1(&scope(), input, &facts(), r),
    )
    .unwrap()
}
#[test]
fn native_selection_handlers_require_sealed_origin_and_exact_contract_operation_selectors() {
    for installed in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        let mut l = limits();
        l.max_context_bytes = 256;
        l.max_packet_bytes = 65536;
        l.max_capture_bytes = 65536;
        l.max_reply_bytes = 65536;
        l.max_row_bytes = 32768;
        l.max_root_bytes = 65536;
        l.metadata.max_bytes = 8192;
        l.metadata.max_nodes = 512;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        o.configure_retention_budget(l.max_held_bytes, &mut r)
            .unwrap();
        o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
            .unwrap();
        let p = prepare(&mut o, installed, &mut r);
        let context = o
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, installed, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: None,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let declarations = native_selection_argument_declarations_v1();
        for (i, entry) in entries.iter().enumerate() {
            let p = prepare(&mut o, installed, &mut r);
            let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                .query_discovery_v1(
                    &context,
                    &scope(),
                    &p,
                    DiscoveryQueryV1::Inspect {
                        entry: &entry.reference,
                    },
                    &facts(),
                    &mut r,
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(descriptor.variants.len(), if i == 0 { 3 } else { 1 });
            for (j, variant) in descriptor.variants.iter().enumerate() {
                for (k, declaration) in declarations.iter().enumerate() {
                    let correct = k == if i == 0 { j } else { 3 };
                    assert_eq!(
                        queries::selection_selector_matches(
                            o.discovery_context(&context).unwrap(),
                            &entry.reference,
                            &variant.reference,
                            declaration
                        )
                        .is_ok(),
                        installed && correct
                    );
                }
                assert!(
                    queries::selection_selector_matches(
                        o.discovery_context(&context).unwrap(),
                        &entries[1 - i].reference,
                        &variant.reference,
                        declarations[0]
                    )
                    .is_err()
                );
            }
        }
    }
}
