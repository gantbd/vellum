use super::*;
use vellum_engine::capture::{CapturedValueV1 as V, InertCaptureV1};
use vellum_engine::instrumentation::Boundary;

pub(super) fn visibility() -> NativeBuiltinDiscoveryVisibilityV1 {
    NativeBuiltinDiscoveryVisibilityV1 {
        annotation: NativeBuiltinEntryVisibilityV1 {
            order: Some(0),
            schema: true,
            presentation: true,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
        history: NativeBuiltinEntryVisibilityV1 {
            order: Some(1),
            schema: true,
            presentation: true,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
    }
}
fn setup_builtin() -> support::Fixture {
    let mut f = support::Fixture::new();
    f.owner
        .configure_retention_budget(4194304, &mut f.recorder)
        .unwrap();
    let mut l = limits();
    l.max_row_bytes = 32768;
    l.max_capture_bytes = 65536;
    l.max_held_bytes = 2097152;
    f.owner
        .configure_discovery_v1(&scope(), l, &facts(), &mut f.recorder)
        .unwrap();
    f
}
fn builtin_cut(
    f: &mut support::Fixture,
    v: &NativeBuiltinDiscoveryVisibilityV1,
) -> DiscoveryCutIdV1 {
    f.owner
        .prepare_native_builtin_discovery_cut_v1(
            &scope(),
            v,
            None,
            false,
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
}
fn builtin_query(
    f: &mut support::Fixture,
    id: &DiscoveryContextIdV1,
    v: &NativeBuiltinDiscoveryVisibilityV1,
    q: DiscoveryQueryV1<'_>,
) -> Result<DiscoveryReplyV1, DiscoveryErrorV1> {
    let cut = builtin_cut(f, v);
    f.owner
        .query_discovery_v1(id, &scope(), &cut, q, &facts(), &mut f.recorder)
}
fn member<'a>(c: &'a InertCaptureV1, node: &'a V, name: &str) -> &'a V {
    let V::Object(rows) = node else {
        panic!("object")
    };
    c.node(rows.iter().find(|(k, _)| k == name).unwrap().1)
        .unwrap()
}
fn assert_text(v: &V, expected: &str) {
    assert!(matches!(v, V::Text(s) if s == expected));
}

#[test]
fn builtin_declarations_are_the_six_closed_native_operations_and_all_five_facets_are_released() {
    let mut f = setup_builtin();
    let generation = f.owner.generation();
    let issued = f.owner.invocation_usage();
    let rows = f
        .owner
        .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        rows[0].declaration,
        native_annotation_authoring_declaration_v1()
    );
    assert_eq!(
        rows[1].declaration,
        native_history_reversal_declaration_v1()
    );
    assert_ne!(rows[0].command, rows[1].command);
    assert_eq!(rows[0].authority, scope().context.authority);
    let v = visibility();
    let cut = builtin_cut(&mut f, &v);
    let context = f
        .owner
        .acquire_discovery_v1(&scope(), &cut, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let DiscoveryReplyV1::Page { entries, .. } = builtin_query(
        &mut f,
        &context,
        &v,
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!("page")
    };
    assert_eq!(entries.len(), 2);
    for (entry, row) in entries.iter().zip(&rows) {
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = builtin_query(
            &mut f,
            &context,
            &v,
            DiscoveryQueryV1::Inspect {
                entry: &entry.reference,
            },
        )
        .unwrap() else {
            panic!("descriptor")
        };
        assert_eq!(descriptor.kind, DiscoveryExecutionKindV1::LocalCommand);
        assert_eq!(descriptor.variants.len(), row.declaration.variants.len());
        for (actual, declared) in descriptor.variants.iter().zip(row.declaration.variants) {
            let c = actual.schema.as_ref().unwrap();
            let V::Object(facets) = c.root() else {
                panic!("facets")
            };
            assert_eq!(facets.len(), 5);
            let input = member(c, c.root(), "input");
            assert_text(member(c, input, "contract"), row.declaration.contract);
            assert_text(member(c, input, "variant"), declared.operation);
            let fields = member(c, input, "fields");
            let V::Object(actual_fields) = fields else {
                panic!("fields")
            };
            assert_eq!(actual_fields.len(), declared.fields.len());
            for (name, kind) in declared.fields {
                assert_text(member(c, fields, name), kind);
            }
            for facet in ["outcomes", "requirements", "limits", "cancellation"] {
                member(c, c.root(), facet);
            }
        }
    }
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.invocation_usage(), issued);
}

#[test]
fn builtin_origin_discharges_only_the_two_source_premises() {
    for field in 0..10 {
        for value in [None, Some(false)] {
            let mut f = setup_builtin();
            let mut g = facts();
            match field {
                0 => g.accepted_registry_attested = value,
                1 => g.schema_descriptions_attested = value,
                2 => g.authorized = value,
                3 => g.interpretation_current = value,
                4 => g.visibility_attested = value,
                5 => g.visibility_complete = value,
                6 => g.projection_order_attested = value,
                7 => g.outside_publication_guard = value,
                8 => g.resources_ready = value,
                _ => g.preparation_reserved = value,
            }
            let result = f.owner.prepare_native_builtin_discovery_cut_v1(
                &scope(),
                &visibility(),
                None,
                false,
                &g,
                &mut f.recorder,
            );
            assert_eq!(result.is_ok(), field < 2);
            if field >= 2 {
                assert_eq!(f.owner.discovery_usage_v1().unwrap().preparations, 0);
            }
        }
    }
}

#[test]
fn fresh_builtin_metadata_survives_unrelated_edits_but_stale_cuts_do_not_transfer() {
    let mut f = setup_builtin();
    let v = visibility();
    let p = builtin_cut(&mut f, &v);
    let context = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let stale = builtin_cut(&mut f, &v);
    f.owner.create(b"unrelated prose", &mut f.recorder).unwrap();
    assert!(
        f.owner
            .query_discovery_v1(
                &context,
                &scope(),
                &stale,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: None
                },
                &facts(),
                &mut f.recorder
            )
            .is_err()
    );
    assert_eq!(f.owner.discovery_usage_v1().unwrap().preparations, 1);
    f.owner
        .retire_discovery_cut_v1(&stale, &facts(), &mut f.recorder)
        .unwrap();
    assert!(
        builtin_query(
            &mut f,
            &context,
            &v,
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None
            }
        )
        .is_ok()
    );
}

#[test]
fn builtin_capture_ceiling_does_not_widen_and_known_metadata_loss_stops_it() {
    let mut f = setup_builtin();
    let mut v = visibility();
    v.history.order = None;
    v.history.schema = false;
    v.history.presentation = false;
    for variant in &mut v.history.variants {
        variant.order = None;
    }
    let p = builtin_cut(&mut f, &v);
    let context = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    let DiscoveryReplyV1::Page { entries, .. } = builtin_query(
        &mut f,
        &context,
        &visibility(),
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!("page")
    };
    assert_eq!(entries.len(), 1);
    v.annotation.schema = false;
    assert!(
        builtin_query(
            &mut f,
            &context,
            &v,
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None
            }
        )
        .is_err()
    );
    assert!(
        builtin_query(
            &mut f,
            &context,
            &visibility(),
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None
            }
        )
        .is_err()
    );
}

#[test]
fn builtin_diagnostics_are_optional_bounded_and_do_not_change_the_descriptor_path() {
    for capacity in [0, 1, 128] {
        let mut f = setup_builtin();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(
                vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        let rows = f
            .owner
            .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder)
            .unwrap();
        assert_eq!(rows.len(), 2);
        let p = builtin_cut(&mut f, &visibility());
        let context = f
            .owner
            .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
            .unwrap()
            .context;
        builtin_query(
            &mut f,
            &context,
            &visibility(),
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None,
            },
        )
        .unwrap();
        assert!(f.recorder.queued() <= capacity);
        if capacity == 128 {
            let mut boundaries = Vec::new();
            while let Some(o) = f.recorder.pop_front() {
                if [
                    Boundary::DiscoveryBuiltinInspection,
                    Boundary::DiscoveryBuiltinPreparation,
                ]
                .contains(&o.boundary)
                {
                    assert!(o.service_time.is_some());
                }
                boundaries.push(o.boundary);
            }
            for b in [
                Boundary::DiscoveryBuiltinInspection,
                Boundary::DiscoveryBuiltinPreparation,
                Boundary::DiscoveryPreparation,
                Boundary::DiscoveryAcquisition,
                Boundary::DiscoveryQuery,
            ] {
                assert!(boundaries.contains(&b));
            }
        }
    }
}

#[test]
fn builtin_source_never_returns_a_partial_catalog_when_entry_or_variant_capacity_is_missing() {
    for (entries, variants) in [(1, 4), (2, 3)] {
        let mut f = support::Fixture::new();
        f.owner
            .configure_retention_budget(4194304, &mut f.recorder)
            .unwrap();
        let mut l = limits();
        l.max_entries = entries;
        l.max_variants = variants;
        f.owner
            .configure_discovery_v1(&scope(), l, &facts(), &mut f.recorder)
            .unwrap();
        assert!(matches!(
            f.owner
                .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder),
            Err(DiscoveryErrorV1::Limit)
        ));
        assert!(matches!(
            f.owner.prepare_native_builtin_discovery_cut_v1(
                &scope(),
                &visibility(),
                None,
                false,
                &facts(),
                &mut f.recorder
            ),
            Err(DiscoveryErrorV1::Limit)
        ));
        assert_eq!(f.owner.discovery_usage_v1().unwrap().preparations, 0);
    }
}

#[test]
fn builtin_owner_inspection_is_not_an_authority_bypass_and_copies_do_not_change_the_source() {
    let mut f = setup_builtin();
    for value in [None, Some(false)] {
        let mut g = facts();
        g.owner_control_allowed = value;
        assert!(
            f.owner
                .native_builtin_discovery_rows_v1(&scope(), &g, &mut f.recorder)
                .is_err()
        );
    }
    let mut row = f
        .owner
        .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder)
        .unwrap();
    row[0].command.clear();
    row[0].authority.clear();
    let original = f
        .owner
        .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder)
        .unwrap();
    assert!(!original[0].command.is_empty());
    assert_eq!(original[0].authority, scope().context.authority);
    let mut foreign = scope();
    foreign.context.engine.push_str("foreign");
    assert!(
        f.owner
            .native_builtin_discovery_rows_v1(&foreign, &facts(), &mut f.recorder)
            .is_err()
    );
}

#[test]
fn builtin_context_uses_existing_protected_retirement_and_never_refunds_lifetime_issuance() {
    let mut f = setup_builtin();
    let parent = f.owner.retention_usage().reserved;
    let p = builtin_cut(&mut f, &visibility());
    let context = f
        .owner
        .acquire_discovery_v1(&scope(), &p, &facts(), &mut f.recorder)
        .unwrap()
        .context;
    builtin_query(
        &mut f,
        &context,
        &visibility(),
        DiscoveryQueryV1::List {
            limit: 2,
            continuation: None,
        },
    )
    .unwrap();
    let issued = f.owner.discovery_usage_v1().unwrap().issued_lifetime;
    f.owner
        .stop_discovery_context_v1(&context, &scope(), &facts(), &mut f.recorder)
        .unwrap();
    let mut unknown = facts();
    unknown.delivery_drained = None;
    assert!(
        f.owner
            .retire_discovery_context_v1(&context, &unknown, &mut f.recorder)
            .is_err()
    );
    f.owner
        .retire_discovery_context_v1(&context, &facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.discovery_usage_v1().unwrap().issued_lifetime,
        issued
    );
    assert_eq!(f.owner.retention_usage().reserved, parent);
    f.owner
        .stop_discovery_admission_v1(&facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_discovery_registry_v1(&facts(), &mut f.recorder)
        .unwrap();
    assert!(f.owner.retention_usage().reserved < parent);
    assert!(
        f.owner
            .native_builtin_discovery_rows_v1(&scope(), &facts(), &mut f.recorder)
            .is_err()
    );
}
