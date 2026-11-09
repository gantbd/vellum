use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
fn visibility() -> NativeBuiltinEntryVisibilityV1<1> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: [NativeBuiltinVariantVisibilityV1 {
            order: Some(0),
            enabled: None,
        }],
    }
}
fn prepare(o: &mut SourceCoordinator, builtin: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if builtin {
        return o
            .prepare_native_rebind_discovery_cut_v1(&scope(), &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F17-source-rebind",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |input| {
        o.prepare_discovery_cut_v1(&scope(), input, &facts(), r)
    })
    .unwrap()
}
#[test]
fn identical_source_rebind_metadata_cannot_claim_installed_source_or_poison_it() {
    for builtin in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        o.configure_retention_budget(4194304, &mut r).unwrap();
        let mut l = limits();
        l.max_row_bytes = 32768;
        l.max_packet_bytes = 65536;
        l.max_capture_bytes = 65536;
        l.max_reply_bytes = 65536;
        l.max_held_bytes = 2097152;
        l.metadata.max_nodes = 1024;
        o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
            .unwrap();
        let p = prepare(&mut o, builtin, &mut r);
        let context = o
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, builtin, &mut r);
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
            panic!()
        };
        let entry = &entries[0].reference;
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let variant = &descriptor.variants[0].reference;
        let c = o.discovery_context(&context).unwrap();
        assert_eq!(
            queries::rebind_selector_matches(c, entry, variant).is_ok(),
            builtin
        );
        assert!(queries::container_selector_matches(c, entry, variant, "view-container").is_err());
        assert!(queries::lifecycle_selector_matches(c, entry, variant, "dispose").is_err());
        let p = prepare(&mut o, !builtin, &mut r);
        let mut lost = facts();
        lost.authorized = Some(false);
        assert_eq!(
            o.query_discovery_v1(
                &context,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect { entry },
                &lost,
                &mut r
            )
            .err(),
            Some(Error::Unavailable)
        );
        o.retire_discovery_cut_v1(&p, &facts(), &mut r).unwrap();
        let p = prepare(&mut o, builtin, &mut r);
        o.query_discovery_v1(
            &context,
            &scope(),
            &p,
            DiscoveryQueryV1::Inspect { entry },
            &facts(),
            &mut r,
        )
        .unwrap();
    }
}
