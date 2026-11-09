//! Selector-only origin test; actual gateway readiness/dispatch is covered by
//! the public diagnostic-drain consumer, not assumed by this fixture.
use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits, scope};
fn prepare(o: &mut SourceCoordinator, sealed: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F26-diagnostic-drain-v1",
        store.limits.max_context_bytes,
    )
    .unwrap();
    selection::with_variant(
        native_diagnostic_drain_argument_declaration_v1(),
        NativeBuiltinVariantVisibilityV1 {
            order: Some(0),
            enabled: None,
        },
        |variant| {
            let variants = [variant];
            let entries = [DiscoveryEntryInputV1 {
                command: DiscoveryCommandV1 {
                    authority: &authority,
                    id: &command,
                    incarnation: 1,
                },
                contract: native_diagnostic_drain_argument_declaration_v1().contract,
                version: 1,
                kind: DiscoveryExecutionKindV1::EffectRequest,
                variants: &variants,
                presentation: D::Object(&[]),
                order: Some(0),
                schema_visible: true,
                presentation_visible: false,
                advice_visible: false,
            }];
            let input = DiscoveryCutInputV1 {
                entries: &entries,
                target: None,
                advice_present: false,
            };
            if sealed {
                o.prepare_discovery_cut_from_source(
                    &scope(),
                    input,
                    &facts(),
                    Origin::NativeDiagnosticDrainV1,
                    r,
                )
            } else {
                o.prepare_discovery_cut_v1(&scope(), input, &facts(), r)
            }
            .unwrap()
        },
    )
}
#[test]
fn matching_contract_kind_operation_and_argument_schema_do_not_install_a_handler() {
    for sealed in [false, true] {
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
        let cut = prepare(&mut o, sealed, &mut r);
        let context = o
            .acquire_discovery_v1(&scope(), &cut, &facts(), &mut r)
            .unwrap()
            .context;
        let cut = prepare(&mut o, sealed, &mut r);
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
        let cut = prepare(&mut o, sealed, &mut r);
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
        assert_eq!(
            queries::diagnostic_drain_selector_matches(
                o.discovery_context(&context).unwrap(),
                &entries[0].reference,
                &descriptor.variants[0].reference
            )
            .is_ok(),
            sealed
        );
    }
}
