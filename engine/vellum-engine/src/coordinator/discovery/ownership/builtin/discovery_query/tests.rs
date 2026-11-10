use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};

fn visibility() -> NativeBuiltinEntryVisibilityV1<3> {
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
fn prepare(o: &mut SourceCoordinator, sealed: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if sealed {
        return o
            .prepare_native_discovery_query_cut_v1(&scope(), &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F01-discovery-query-v1",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |input| {
        o.prepare_discovery_cut_v1(&scope(), input, &facts(), r)
    })
    .unwrap()
}
#[test]
fn identical_query_metadata_cannot_install_a_discovery_handler_or_bypass_original_selectors() {
    for sealed in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        let mut l = limits();
        l.max_context_bytes = 256;
        l.max_row_bytes = 65536;
        l.max_capture_bytes = 131072;
        l.max_packet_bytes = 131072;
        l.max_reply_bytes = 131072;
        l.metadata.max_bytes = 32768;
        l.metadata.max_nodes = 1024;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        o.configure_retention_budget(l.max_held_bytes, &mut r)
            .unwrap();
        o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
            .unwrap();
        let p = prepare(&mut o, sealed, &mut r);
        let context = o
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, sealed, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!("page")
        };
        let p = prepare(&mut o, sealed, &mut r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect {
                    entry: &entries[0].reference,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!("descriptor")
        };
        for (i, d) in native_discovery_input_forms_v1().iter().enumerate() {
            assert_eq!(
                queries::discovery_query_selector_matches(
                    o.discovery_context(&context).unwrap(),
                    &entries[0].reference,
                    &descriptor.variants[i].reference,
                    d.operation,
                )
                .is_ok(),
                sealed
            );
            assert!(
                queries::discovery_query_selector_matches(
                    o.discovery_context(&context).unwrap(),
                    &entries[0].reference,
                    &descriptor.variants[(i + 1) % 3].reference,
                    d.operation,
                )
                .is_err()
            );
        }
        o.stop_discovery_context_v1(&context, &scope(), &facts(), &mut r)
            .unwrap();
        o.retire_discovery_context_v1(&context, &facts(), &mut r)
            .unwrap();
        o.stop_discovery_admission_v1(&facts(), &mut r).unwrap();
        o.retire_discovery_registry_v1(&facts(), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
