use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
fn visibility() -> NativeBuiltinEntryVisibilityV1<2> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
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
            .prepare_native_replacement_discovery_cut_v1(&scope(), &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F11-replacement-v1",
        store.limits.max_context_bytes,
    )
    .unwrap();
    let declarations = native_replacement_argument_declarations_v1();
    selection::with_variant(&declarations[0], visibility().variants[0], |current| {
        selection::with_variant(&declarations[1], visibility().variants[1], |all| {
            let variants = [describe(current), describe(all)];
            let entries = [DiscoveryEntryInputV1 {
                command: DiscoveryCommandV1 {
                    authority: &authority,
                    id: &command,
                    incarnation: 1,
                },
                contract: declarations[0].contract,
                version: 1,
                kind: DiscoveryExecutionKindV1::LocalCommand,
                variants: &variants,
                presentation: D::Object(&[]),
                order: Some(0),
                schema_visible: true,
                presentation_visible: false,
                advice_visible: false,
            }];
            o.prepare_discovery_cut_v1(
                &scope(),
                DiscoveryCutInputV1 {
                    entries: &entries,
                    target: None,
                    advice_present: false,
                },
                &facts(),
                r,
            )
            .unwrap()
        })
    })
}
#[test]
fn copied_replacement_metadata_has_no_handler_and_each_selector_keeps_its_operation() {
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
        let cut = prepare(&mut o, installed, &mut r);
        let context = o
            .acquire_discovery_v1(&scope(), &cut, &facts(), &mut r)
            .unwrap()
            .context;
        let cut = prepare(&mut o, installed, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &cut,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let cut = prepare(&mut o, installed, &mut r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &cut,
                DiscoveryQueryV1::Inspect {
                    entry: &entries[0].reference,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(descriptor.variants.len(), 2);
        for (i, variant) in descriptor.variants.iter().enumerate() {
            for (j, declaration) in native_replacement_argument_declarations_v1()
                .iter()
                .enumerate()
            {
                assert_eq!(
                    queries::replacement_selector_matches(
                        o.discovery_context(&context).unwrap(),
                        &entries[0].reference,
                        &variant.reference,
                        declaration
                    )
                    .is_ok(),
                    installed && i == j
                );
            }
        }
    }
}
