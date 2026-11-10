use super::*;
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::selection::Selection;
fn fixture() -> (SourceCoordinator, LogicalViewContext, Recorder) {
    let (mut o, mut r) = owner();
    relations::configure(&mut o, &mut r);
    o.configure_line_edits(
        LineEditBudget {
            max_lines: 16,
            max_prefix_bytes: 1024,
            max_indent_columns: 128,
            max_unit_bytes: 8,
            max_derived_bytes: 4096,
            max_source_scan_bytes: 1024,
        },
        &mut r,
    )
    .unwrap();
    let d = o.create(b"abc", &mut r).unwrap();
    let view = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
    o.attach_logical_view(
        &view,
        stamp,
        &d,
        &[Selection {
            anchor: BytePosition(0),
            head: BytePosition(0),
            affinity: Affinity::After,
        }],
        0,
        &mut r,
    )
    .unwrap();
    let c = o.logical_view(&view, &mut r).unwrap().context().unwrap();
    (o, c, r)
}
fn submit(
    o: &mut SourceCoordinator,
    command: NativeDiscoveredOrdinaryEditV1<'_, '_>,
    r: &mut Recorder,
) {
    let (dc, e, v) = selectors_for(o, r, 3);
    let p = prepare(o, r);
    assert_eq!(
        o.submit_discovered_native_ordinary_edit_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[0].reference,
                facts: &facts()
            },
            command,
            r
        ),
        Ok(InvocationState::Pending)
    );
}
#[test]
fn installed_ordinary_literal_dispatch_uses_original_native_owner_and_replay() {
    use LiteralEditEnvelopeValueV1 as V;
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 256] {
        let (mut o, mut r) = owner();
        relations::configure(&mut o, &mut r);
        let d = o.create(b"old", &mut r).unwrap();
        let view = o.create_logical_view(&mut r).unwrap();
        let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        o.attach_logical_view(
            &view,
            stamp,
            &d,
            &[Selection {
                anchor: BytePosition(0),
                head: BytePosition(3),
                affinity: Affinity::After,
            }],
            0,
            &mut r,
        )
        .unwrap();
        let context = o.logical_view(&view, &mut r).unwrap().context().unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let raw = [
            ("contract", V::Data(D::Text("selection-edit/1"))),
            ("operation", V::Data(D::Text("ordinary-edit"))),
            ("invocation", V::Invocation(&id)),
            ("view", V::View(&context.view)),
            ("document", V::Document(&context.source.document)),
            (
                "sourceRevision",
                V::Data(D::Number(context.source.revision as f64)),
            ),
            (
                "expectedSelection",
                V::Data(D::Number(context.stamp.selection_generation as f64)),
            ),
            (
                "profile",
                V::Data(D::Text(EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1)),
            ),
            ("text", V::Data(D::Text("Deutsch\n```mermaid\n```"))),
            ("selectionAfter", V::Data(D::Text("carets-after"))),
        ];
        let n = normalize_literal_edit_arguments_v1(
            V::Object(&raw),
            &context,
            LiteralEditArgumentLimitsV1 {
                max_text_bytes: 4096,
                max_owned_bytes: 8192,
            },
            &mut r,
        )
        .unwrap();
        let (dc, e, v) = selectors_for(&mut o, &mut r, 3);
        let p = prepare(&mut o, &mut r);
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        assert_eq!(
            o.submit_discovered_native_ordinary_edit_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &dc,
                    scope: &scope4(),
                    cut: &p,
                    entry: &e,
                    variant: &v[0].reference,
                    facts: &facts()
                },
                NativeDiscoveredOrdinaryEditV1::Literal(&n),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        o.run(&id, &mut r).unwrap();
        let generation = o.generation();
        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), n.text());
        let p = prepare(&mut o, &mut r);
        assert_eq!(
            o.submit_discovered_native_ordinary_edit_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &dc,
                    scope: &scope4(),
                    cut: &p,
                    entry: &e,
                    variant: &v[0].reference,
                    facts: &facts()
                },
                NativeDiscoveredOrdinaryEditV1::Literal(&n),
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        for case in 0..3 {
            let new_id = o.issue_invocation(&mut r).unwrap();
            let mut other = raw;
            other[2].1 = V::Invocation(&new_id);
            let other = normalize_literal_edit_arguments_v1(
                V::Object(&other),
                &context,
                LiteralEditArgumentLimitsV1 {
                    max_text_bytes: 4096,
                    max_owned_bytes: 8192,
                },
                &mut r,
            )
            .unwrap();
            let p = prepare(&mut o, &mut r);
            let mut scope = scope4();
            let mut current = facts();
            if case == 0 {
                o.create(b"unrelated root change", &mut r).unwrap();
            }
            if case == 1 {
                scope.context.recipient = "foreign".into();
                current.authorized = Some(false);
            }
            if case == 2 {
                current.authorized = Some(false);
            }
            let generation = o.generation();
            assert_eq!(
                o.submit_discovered_native_ordinary_edit_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope,
                        cut: &p,
                        entry: &e,
                        variant: &v[0].reference,
                        facts: &current
                    },
                    NativeDiscoveredOrdinaryEditV1::Literal(&other),
                    &mut r
                ),
                Err(DiscoveredNativeCommandErrorV1::Discovery(
                    DiscoveryErrorV1::Unavailable
                ))
            );
            assert_eq!(
                o.invocation_state(&new_id, &mut r),
                Ok(InvocationState::Issued)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(
                o.discovery_context_usage_v1(&dc).unwrap().stopped,
                case == 2
            );
        }
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 256 {
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::DiscoveryQuery && v.service_time.is_some())
            );
            assert!(records.iter().any(
                |v| v.boundary == Boundary::LiteralDecodedAdmission && v.service_time.is_some()
            ));
        } else {
            assert!(records.len() <= capacity);
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
}

#[test]
fn installed_explicit_line_dispatch_applies_the_existing_prefix_owner() {
    use LineEditEnvelopeValueV1 as V;
    let (mut o, c, mut r) = fixture();
    let id = o.issue_invocation(&mut r).unwrap();
    let settings = [("unit", D::Text("  ")), ("tabWidth", D::Number(4.0))];
    let raw = [
        ("contract", V::Data(D::Text("line-edit/1"))),
        ("operation", V::Data(D::Text("ordinary-edit"))),
        ("action", V::Data(D::Text("indent-lines"))),
        ("invocation", V::Invocation(&id)),
        ("view", V::View(&c.view)),
        ("document", V::Document(&c.source.document)),
        (
            "sourceRevision",
            V::Data(D::Number(c.source.revision as f64)),
        ),
        (
            "expectedSelection",
            V::Data(D::Number(c.stamp.selection_generation as f64)),
        ),
        (
            "profile",
            V::Data(D::Text(EXPLICIT_PREFIX_LINE_EDIT_PROFILE_V1)),
        ),
        ("settings", V::Data(D::Object(&settings))),
    ];
    let n = normalize_line_edit_arguments_v1(
        V::Object(&raw),
        &c,
        LineEditArgumentLimitsV1 {
            max_unit_bytes: 8,
            max_owned_bytes: 8192,
        },
        &mut r,
    )
    .unwrap();
    submit(&mut o, NativeDiscoveredOrdinaryEditV1::Line(&n), &mut r);
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.read(&c.source.document, &mut r).unwrap().source.text(),
        "  abc"
    );
}

#[test]
fn installed_boundary_dispatch_uses_original_native_evidence_without_resegmenting() {
    use BoundaryEditEnvelopeValueV1 as V;
    let (mut o, c, mut r) = fixture();
    o.configure_boundaries(
        BoundaryLimits {
            max_issued: 8,
            max_retained: 4,
            max_source_bytes: 2048,
            max_boundaries: 64,
            max_words: 8,
            max_evidence_bytes: 4096,
            max_retained_bytes: 8192,
        },
        &mut r,
    )
    .unwrap();
    o.configure_unicode_commands(
        UnicodeCommandBudget {
            max_steps: 64,
            max_derived_bytes: 4096,
        },
        &mut r,
    )
    .unwrap();
    let capture = o.begin_boundary_capture(&c.source, &mut r).unwrap();
    let e = o
        .accept_boundary_evidence(
            &capture,
            &BoundaryTableV1 {
                profile: BoundaryProfileV1::Unicode18Uax29Revision49,
                graphemes: (0..=3).map(BytePosition).collect(),
                words: vec![BytePosition(0), BytePosition(3)],
                significant_words: vec![ByteRange { start: 0, end: 3 }],
            },
            &mut r,
        )
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = [
        ("contract", V::Data(D::Text("boundary-command/1"))),
        ("operation", V::Data(D::Text("ordinary-edit"))),
        ("invocation", V::Invocation(&id)),
        ("view", V::View(&c.view)),
        ("document", V::Document(&c.source.document)),
        (
            "sourceRevision",
            V::Data(D::Number(c.source.revision as f64)),
        ),
        (
            "expectedSelection",
            V::Data(D::Number(c.stamp.selection_generation as f64)),
        ),
        ("profile", V::Data(D::Text(UNICODE_LOGICAL_EDIT_PROFILE_V1))),
        ("boundary", V::Boundary(&e)),
        ("unit", V::Data(D::Text("grapheme"))),
        ("direction", V::Data(D::Text("forward"))),
        ("count", V::Data(D::Number(1.0))),
        ("extend", V::Data(D::Boolean(false))),
    ];
    let n = normalize_boundary_edit_arguments_v1(
        V::Object(&raw),
        &c,
        BoundaryEditArgumentLimitsV1 {
            max_owned_bytes: 8192,
        },
        &mut r,
    )
    .unwrap();
    submit(&mut o, NativeDiscoveredOrdinaryEditV1::Boundary(&n), &mut r);
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.read(&c.source.document, &mut r).unwrap().source.text(),
        "bc"
    );
}

#[test]
fn installed_configured_line_dispatch_retains_the_original_settings_interpretation() {
    use ConfiguredLineEnvelopeValueV1 as V;
    use vellum_engine::settings::*;
    let (mut o, c, mut r) = fixture();
    let identity = |s: &str| SettingIdentityV1 {
        authority: "engine".into(),
        id: s.into(),
        incarnation: 1,
    };
    let definition = SettingsDefinitionV1 {
        schemas: vec![
            SettingSchemaV1 {
                key: "prefix".into(),
                value_type: SettingTypeV1::Text,
                default: SettingValueV1::Text("  ".into()),
                domain: SettingDomainV1::Source,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::Replace,
                sensitive: false,
            },
            SettingSchemaV1 {
                key: "columns".into(),
                value_type: SettingTypeV1::Integer { min: 0, max: 16 },
                default: SettingValueV1::Number(4.0),
                domain: SettingDomainV1::Source,
                scopes: vec![SettingScopeV1::Host],
                composition: SettingCompositionV1::Replace,
                sensitive: false,
            },
        ],
        restrictions: vec![],
        layers: vec![SettingLayerV1 {
            identity: identity("host"),
            scope: SettingScopeV1::Host,
            generation: 1,
            values: vec![],
        }],
        targets: vec![SettingTargetV1 {
            identity: identity("A"),
            layers: vec![identity("host")],
        }],
    };
    o.install_initial_settings(
        compile_settings_v1(
            &definition,
            "engine",
            SettingsLimitsV1 {
                max_keys: 16,
                max_layers: 8,
                max_targets: 8,
                max_identifier_bytes: 64,
                max_schema_nodes: 128,
                max_schema_depth: 32,
                max_value_nodes: 256,
                max_value_depth: 32,
                max_state_bytes: 65536,
                max_work: 1048576,
            },
            &mut r,
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 16,
            max_request_nodes: 128,
            max_request_depth: 32,
            max_request_bytes: 65536,
            max_plan_bytes: 262144,
            max_work: 1048576,
        },
        &mut r,
    )
    .unwrap();
    o.configure_settings_retention(
        SettingsRetentionLimitsV1 {
            max_issued: 16,
            max_retained: 8,
            max_snapshot_bytes: 65536,
            max_pool_bytes: 1048576,
        },
        &mut r,
    )
    .unwrap();
    o.configure_configured_line_edits(
        ConfiguredLineLimitsV1 {
            max_identifier_bytes: 64,
            max_request_bytes: 4096,
            max_inner_intent_bytes: 4096,
        },
        &mut r,
    )
    .unwrap();
    let target = o.settings_target(&identity("A"), &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let keys = [
        ("unit", D::Text("prefix")),
        ("tabWidth", D::Text("columns")),
    ];
    let raw = [
        ("contract", V::Data(D::Text("configured-line-edit/1"))),
        ("operation", V::Data(D::Text("ordinary-edit"))),
        ("action", V::Data(D::Text("indent-lines"))),
        ("invocation", V::Invocation(&id)),
        ("document", V::Document(&c.source.document)),
        ("view", V::View(&c.view)),
        (
            "sourceRevision",
            V::Data(D::Number(c.source.revision as f64)),
        ),
        (
            "expectedSelection",
            V::Data(D::Number(c.stamp.selection_generation as f64)),
        ),
        (
            "profile",
            V::Data(D::Text(CONFIGURED_PREFIX_LINE_PROFILE_V1)),
        ),
        (
            "configuration",
            V::Configuration {
                target: &target,
                keys: D::Object(&keys),
            },
        ),
    ];
    let n = normalize_configured_line_arguments_v1(
        V::Object(&raw),
        &c,
        ConfiguredLineArgumentLimitsV1 {
            max_key_bytes: 64,
            max_owned_bytes: 8192,
        },
        &mut r,
    )
    .unwrap();
    submit(
        &mut o,
        NativeDiscoveredOrdinaryEditV1::ConfiguredLine(&n),
        &mut r,
    );
    assert_eq!(o.settings_retention_usage().retained, 1);
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.read(&c.source.document, &mut r).unwrap().source.text(),
        "  abc"
    );
}
