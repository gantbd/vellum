//! Source-identity challenge; actual decoder/parent dispatch is covered publicly.
use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
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
fn prepare(o: &mut SourceCoordinator, sealed: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    let s = o.discovery_store().unwrap();
    let authority = s.authority.clone();
    let commands = ["F06-native-operation-v1", "F06-native-operation-receipt-v1"]
        .map(|n| command(&s.engine, n, s.limits.max_context_bytes).unwrap());
    with_input(&authority, &commands, &visible(0), &visible(1), |input| {
        o.prepare_discovery_cut_from_source(
            &scope(),
            input,
            &facts(),
            if sealed {
                Origin::NativeOperationObservationV1
            } else {
                Origin::Generic
            },
            r,
        )
    })
    .unwrap()
}
#[test]
fn matching_descriptions_never_substitute_for_installed_native_operation_identity() {
    for sealed in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        let mut l = limits();
        l.max_context_bytes = 256;
        l.max_capture_bytes = 65536;
        l.max_packet_bytes = 65536;
        l.max_reply_bytes = 65536;
        l.max_row_bytes = 32768;
        l.metadata.max_bytes = 8192;
        l.metadata.max_nodes = 512;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        o.configure_retention_budget(l.max_held_bytes, &mut r)
            .unwrap();
        o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
            .unwrap();
        let p = prepare(&mut o, sealed, &mut r);
        let c = o
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, sealed, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &c,
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
            panic!("page")
        };
        let d = native_operation_argument_declarations_v1();
        for (ei, declarations) in [[d[0], d[2], d[3]].as_slice(), [d[1]].as_slice()]
            .into_iter()
            .enumerate()
        {
            let p = prepare(&mut o, sealed, &mut r);
            let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                .query_discovery_v1(
                    &c,
                    &scope(),
                    &p,
                    DiscoveryQueryV1::Inspect {
                        entry: &entries[ei].reference,
                    },
                    &facts(),
                    &mut r,
                )
                .unwrap()
            else {
                panic!("descriptor")
            };
            for (v, declaration) in descriptor.variants.iter().zip(declarations) {
                assert_eq!(
                    queries::native_operation_selector_matches(
                        o.discovery_context(&c).unwrap(),
                        &entries[ei].reference,
                        &v.reference,
                        declaration
                    )
                    .is_ok(),
                    sealed
                );
                let wrong = if ei == 0 { d[1] } else { d[0] };
                assert!(
                    queries::native_operation_selector_matches(
                        o.discovery_context(&c).unwrap(),
                        &entries[ei].reference,
                        &v.reference,
                        wrong
                    )
                    .is_err()
                );
            }
        }
        o.stop_discovery_context_v1(&c, &scope(), &facts(), &mut r)
            .unwrap();
        o.retire_discovery_context_v1(&c, &facts(), &mut r).unwrap();
        o.stop_discovery_admission_v1(&facts(), &mut r).unwrap();
        o.retire_discovery_registry_v1(&facts(), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
