use super::super::tests::{facts, limits, scope};
use super::*;
#[test]
fn installed_annotation_and_reversal_selectors_reject_generic_lookalikes_and_cross_variant_substitution()
 {
    for builtin in [false, true] {
        let (mut o, mut r) = setup_with_reply(65536);
        let p = prepare(&mut o, builtin, &mut r);
        let c = o
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut r)
            .unwrap()
            .context;
        let p = prepare(&mut o, builtin, &mut r);
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
            panic!()
        };
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &c,
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
            panic!()
        };
        let context = o.discovery_context(&c).unwrap();
        for (index, operation) in ["create", "update-body", "reattach", "delete"]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                queries::annotation_selector_matches(
                    context,
                    &entries[0].reference,
                    &descriptor.variants[index].reference,
                    operation
                )
                .is_ok(),
                builtin
            );
            assert!(
                queries::annotation_selector_matches(
                    context,
                    &entries[0].reference,
                    &descriptor.variants[(index + 1) % 4].reference,
                    operation
                )
                .is_err()
            );
            assert!(
                queries::annotation_selector_matches(
                    context,
                    &entries[1].reference,
                    &descriptor.variants[index].reference,
                    operation
                )
                .is_err()
            );
        }
        let p = prepare(&mut o, builtin, &mut r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &c,
                &scope(),
                &p,
                DiscoveryQueryV1::Inspect {
                    entry: &entries[1].reference,
                },
                &facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let context = o.discovery_context(&c).unwrap();
        for (index, direction) in [HistoryDirection::Undo, HistoryDirection::Redo]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                queries::reversal_selector_matches(
                    context,
                    &entries[1].reference,
                    &descriptor.variants[index].reference,
                    direction
                )
                .is_ok(),
                builtin
            );
            assert!(
                queries::reversal_selector_matches(
                    context,
                    &entries[1].reference,
                    &descriptor.variants[1 - index].reference,
                    direction
                )
                .is_err()
            );
            assert!(
                queries::reversal_selector_matches(
                    context,
                    &entries[0].reference,
                    &descriptor.variants[index].reference,
                    direction
                )
                .is_err()
            );
        }
    }
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV1 {
    NativeBuiltinDiscoveryVisibilityV1 {
        annotation: NativeBuiltinEntryVisibilityV1 {
            order: Some(0),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
        history: NativeBuiltinEntryVisibilityV1 {
            order: Some(1),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
    }
}
fn setup() -> (SourceCoordinator, Recorder) {
    setup_with_reply(limits().max_reply_bytes)
}
fn setup_with_reply(reply: usize) -> (SourceCoordinator, Recorder) {
    let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
    o.configure_retention_budget(4194304, &mut r).unwrap();
    let mut l = limits();
    l.max_row_bytes = 32768;
    l.max_packet_bytes = 65536;
    l.max_capture_bytes = 65536;
    l.max_reply_bytes = reply;
    l.max_held_bytes = 2097152;
    o.configure_discovery_v1(&scope(), l, &facts(), &mut r)
        .unwrap();
    (o, r)
}
fn prepare(o: &mut SourceCoordinator, builtin: bool, r: &mut Recorder) -> DiscoveryCutIdV1 {
    if builtin {
        return o
            .prepare_native_builtin_discovery_cut_v1(
                &scope(),
                &visibility(),
                None,
                false,
                &facts(),
                r,
            )
            .unwrap();
    }
    let rows = o
        .native_builtin_discovery_rows_v1(&scope(), &facts(), r)
        .unwrap();
    let commands = [rows[0].command.clone(), rows[1].command.clone()];
    with_inputs(
        &rows[0].authority,
        &commands,
        &visibility(),
        None,
        false,
        |input| o.prepare_discovery_cut_v1(&scope(), input, &facts(), r),
    )
    .unwrap()
}
#[test]
fn identical_generic_and_builtin_cuts_cannot_replace_each_other_or_poison_an_original() {
    for builtin in [false, true] {
        for known_loss in [false, true] {
            let (mut o, mut r) = setup();
            let original = prepare(&mut o, builtin, &mut r);
            let id = o
                .acquire_discovery_v1(&scope(), &original, &facts(), &mut r)
                .unwrap()
                .context;
            let wrong = prepare(&mut o, !builtin, &mut r);
            let mut g = facts();
            if known_loss {
                g.authorized = Some(false);
            }
            assert!(
                o.query_discovery_v1(
                    &id,
                    &scope(),
                    &wrong,
                    DiscoveryQueryV1::List {
                        limit: 2,
                        continuation: None
                    },
                    &g,
                    &mut r
                )
                .is_err()
            );
            assert!(
                !o.discovery_store().unwrap().contexts[id.slot]
                    .as_ref()
                    .unwrap()
                    .stopped
            );
            assert!(o.discovery_store().unwrap().packets[wrong.slot].is_some());
            o.retire_discovery_cut_v1(&wrong, &facts(), &mut r).unwrap();
            let correct = prepare(&mut o, builtin, &mut r);
            assert!(
                o.query_discovery_v1(
                    &id,
                    &scope(),
                    &correct,
                    DiscoveryQueryV1::List {
                        limit: 2,
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
fn handler_declarations_cover_actual_exhaustive_variant_dispatch_and_fixed_frame_bounds() {
    let (mut o, mut r) = setup();
    let source = o.create(b"text", &mut r).unwrap();
    let target = SourceTarget {
        source,
        target: crate::attachment::AttachmentTarget::ContentRange { start: 0, end: 4 },
        basis: TargetBasis::Before,
    };
    let p = o.prepare_compound(
        &[],
        &[],
        &[AnnotationMutation::Create {
            body: "comment".into(),
            target: target.clone(),
        }],
        &mut r,
    );
    // The metadata source does not require annotation capability enablement.
    assert!(p.is_err());
    let created = AnnotationMutation::Create {
        body: String::new(),
        target,
    };
    assert_eq!(created.declaration_v1().operation, "create");
    assert_eq!(HistoryDirection::Undo.declaration_v1().operation, "undo");
    assert_eq!(HistoryDirection::Redo.declaration_v1().operation, "redo");
    for d in declarations() {
        assert!(d.variants.iter().all(|v| v.fields.len() <= 9));
        assert!(d.admission.len() <= 8 && d.limits.len() <= 8);
    }
    assert!(workspace_bytes(usize::MAX).is_err());
}
