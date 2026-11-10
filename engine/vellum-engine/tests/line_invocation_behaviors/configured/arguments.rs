use super::*;
use ConfiguredLineEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::instrumentation::NativeTimingProfileV1;
fn limits() -> ConfiguredLineArgumentLimitsV1 {
    ConfiguredLineArgumentLimitsV1 {
        max_key_bytes: 64,
        max_owned_bytes: 4096,
    }
}
fn fields<'a>(
    id: &'a InvocationId,
    c: &'a LogicalViewContext,
    target: &'a SettingsTargetId,
    action: &'a str,
    keys: D<'a>,
) -> Vec<(&'a str, V<'a, 'a>)> {
    vec![
        ("contract", V::Data(D::Text("configured-line-edit/1"))),
        ("operation", V::Data(D::Text("ordinary-edit"))),
        ("action", V::Data(D::Text(action))),
        ("invocation", V::Invocation(id)),
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
        ("profile", V::Data(D::Text("configured-prefix-line/1"))),
        ("configuration", V::Configuration { target, keys }),
    ]
}
#[test]
fn configured_envelope_preserves_original_target_and_retained_native_settings_lifecycle() {
    for (action, expected) in [
        ("indent-lines", "   \talpha\r\n"),
        ("outdent-lines", "  alpha\r\n"),
        ("insert-newline", " \r\n \talpha\r\n"),
    ] {
        for capacity in [0, 1, 128] {
            let (mut o, c, target, _) = setup();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let id = o.issue_invocation(&mut r).unwrap();
            let keys = if action == "insert-newline" {
                [
                    ("delimiter", D::Text("delimiter")),
                    ("autoIndent", D::Text("auto")),
                ]
            } else {
                [
                    ("unit", D::Text("prefix")),
                    ("tabWidth", D::Text("columns")),
                ]
            };
            let raw = fields(&id, &c, &target, action, D::Object(&keys));
            let before = o.settings_retention_usage();
            let q = normalize_configured_line_arguments_v1(V::Object(&raw), &c, limits(), &mut r)
                .unwrap();
            assert_eq!(o.settings_retention_usage(), before);
            assert_eq!(q.argument_declaration_v1().variant.fields.len(), 10);
            assert!(!format!("{q:?}").contains("columns"));
            o.submit_configured_line_arguments_v1(&q, &mut r).unwrap();
            assert_eq!(o.settings_retention_usage().retained, 1);
            o.run(&id, &mut r).unwrap();
            assert_eq!(text(&o, &c, &mut r), expected);
            let issued = o.settings_retention_usage().issued_lifetime;
            let generation = o.generation();
            assert_eq!(
                o.submit_configured_line_arguments_v1(&q, &mut r),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.settings_retention_usage().issued_lifetime, issued);
            o.retire_invocation(&id, &mut r).unwrap();
            assert_eq!(o.settings_retention_usage().retained, 0);
        }
    }
}
#[test]
fn configuration_requires_a_real_native_target_and_closed_action_key_roles() {
    let (mut o, c, target, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let keys = [
        ("unit", D::Text("prefix")),
        ("tabWidth", D::Text("columns")),
    ];
    let raw = fields(&id, &c, &target, "indent-lines", D::Object(&keys));
    for i in 0..raw.len() {
        let mut bad = raw.clone();
        bad.remove(i);
        assert!(
            normalize_configured_line_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
        let mut bad = raw.clone();
        bad[i] = raw[(i + 1) % raw.len()];
        assert!(
            normalize_configured_line_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
        let mut bad = raw.clone();
        bad[i].1 = V::Data(D::Text("forged"));
        assert!(
            normalize_configured_line_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
    }
    for keys in [
        D::Null,
        D::Object(&[("unit", D::Text("prefix"))]),
        D::Object(&[("unit", D::Text("prefix")), ("unit", D::Text("columns"))]),
        D::Object(&[("unit", D::Number(0.0)), ("tabWidth", D::Text("columns"))]),
        D::Object(&[("unit", D::Text("")), ("tabWidth", D::Text("columns"))]),
    ] {
        let raw = fields(&id, &c, &target, "indent-lines", keys);
        assert!(
            normalize_configured_line_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err()
        );
    }
    let mut bad = raw.clone();
    bad.push(("keymap", V::Data(D::Null)));
    assert!(normalize_configured_line_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
}
#[test]
fn configured_key_copies_have_exact_bounds_and_checked_combined_size() {
    let (mut o, c, target, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let keys = [
        ("unit", D::Text("prefix")),
        ("tabWidth", D::Text("columns")),
    ];
    let raw = fields(&id, &c, &target, "indent-lines", D::Object(&keys));
    let needed = NormalizedConfiguredLineArgumentsV1::required_owned_bytes(6, 7).unwrap();
    for delta in [-1, 0, 1] {
        let l = ConfiguredLineArgumentLimitsV1 {
            max_key_bytes: 7,
            max_owned_bytes: needed.checked_add_signed(delta).unwrap(),
        };
        assert_eq!(
            normalize_configured_line_arguments_v1(V::Object(&raw), &c, l, &mut r).is_ok(),
            delta >= 0
        );
    }
    let q = normalize_configured_line_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    assert_eq!(q.owned_bytes(), needed);
    assert!(
        normalize_configured_line_arguments_v1(
            V::Object(&raw),
            &c,
            ConfiguredLineArgumentLimitsV1 {
                max_key_bytes: 6,
                ..limits()
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        NormalizedConfiguredLineArgumentsV1::required_owned_bytes(usize::MAX, 1),
        Err(InvocationError::Limit)
    );
}
#[test]
fn normalization_does_not_resolve_settings_or_repair_a_stale_native_preparation() {
    let (mut o, c, target, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let keys = [
        ("unit", D::Text("prefix")),
        ("tabWidth", D::Text("columns")),
    ];
    let raw = fields(&id, &c, &target, "indent-lines", D::Object(&keys));
    let q = normalize_configured_line_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    o.submit_configured_line_arguments_v1(&q, &mut r).unwrap();
    set(
        &mut o,
        "left",
        "prefix",
        SettingValueV1::Text("\t".into()),
        &mut r,
    );
    let before = o.generation();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::ConfiguredLineRejected(_)
    ));
    assert_eq!(o.generation(), before);
    assert_eq!(text(&o, &c, &mut r), " \talpha\r\n");
}

#[test]
fn native_target_and_source_domain_truth_are_not_inferred_from_syntactic_key_names() {
    for case in 0..3 {
        let (mut o, c, target, mut r) = setup();
        let (_, _, foreign, _) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        let target = if case == 0 { &foreign } else { &target };
        let keys = [
            (
                "unit",
                D::Text(if case == 1 {
                    "analysis"
                } else if case == 2 {
                    "missing"
                } else {
                    "prefix"
                }),
            ),
            ("tabWidth", D::Text("columns")),
        ];
        let raw = fields(&id, &c, target, "indent-lines", D::Object(&keys));
        let q =
            normalize_configured_line_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        let before = o.generation();
        o.submit_configured_line_arguments_v1(&q, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::ConfiguredLineRejected(_)
        ));
        assert_eq!(o.generation(), before);
        assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
        assert_eq!(text(&o, &c, &mut r), " \talpha\r\n");
    }
}

#[test]
fn configured_normalization_and_admission_have_optional_content_free_timed_dispositions() {
    use vellum_engine::instrumentation::Disposition;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        let (mut o, c, target, _) = setup();
        let mut r = Recorder::try_with_capacity(128)
            .unwrap()
            .with_native_timing(timing);
        let id = o.issue_invocation(&mut r).unwrap();
        let keys = [
            ("unit", D::Text("prefix")),
            ("tabWidth", D::Text("columns")),
        ];
        let raw = fields(&id, &c, &target, "indent-lines", D::Object(&keys));
        let q =
            normalize_configured_line_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        o.submit_configured_line_arguments_v1(&q, &mut r).unwrap();
        let bad = fields(&id, &c, &target, "unknown", D::Object(&keys));
        assert!(
            normalize_configured_line_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
        let changed = fields(&id, &c, &target, "outdent-lines", D::Object(&keys));
        let changed =
            normalize_configured_line_arguments_v1(V::Object(&changed), &c, limits(), &mut r)
                .unwrap();
        assert_eq!(
            o.submit_configured_line_arguments_v1(&changed, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|v| {
                matches!(
                    v.boundary,
                    Boundary::ConfiguredLineArgumentNormalization
                        | Boundary::ConfiguredLineDecodedAdmission
                )
            })
            .collect();
        assert_eq!(records.len(), 5);
        assert_eq!(
            records
                .iter()
                .filter(|v| v.disposition == Disposition::Refused)
                .count(),
            2
        );
        assert!(
            records.iter().all(|v| v.service_time.is_some()
                == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
        );
        assert!(!format!("{records:?}").contains("columns"));
    }
}
