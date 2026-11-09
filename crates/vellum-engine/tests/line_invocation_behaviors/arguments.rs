use super::*;
use LineEditEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::instrumentation::NativeTimingProfileV1;
fn limits() -> LineEditArgumentLimitsV1 {
    LineEditArgumentLimitsV1 {
        max_unit_bytes: 8,
        max_owned_bytes: 4096,
    }
}
fn fields<'a>(
    id: &'a InvocationId,
    c: &'a LogicalViewContext,
    action: &'a str,
    settings: D<'a>,
) -> Vec<(&'a str, V<'a, 'a>)> {
    vec![
        ("contract", V::Data(D::Text("line-edit/1"))),
        ("operation", V::Data(D::Text("ordinary-edit"))),
        ("action", V::Data(D::Text(action))),
        ("invocation", V::Invocation(id)),
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
        ("profile", V::Data(D::Text("explicit-prefix-line-edit/1"))),
        ("settings", V::Data(settings)),
    ]
}
#[test]
fn decoded_prefix_actions_match_native_execution_and_replay_uses_original_settings() {
    for action in ["indent-lines", "outdent-lines"] {
        let (mut o, c, mut r) = fixture(4096);
        let (mut native, nc, mut nr) = fixture(4096);
        let id = o.issue_invocation(&mut r).unwrap();
        let nid = native.issue_invocation(&mut nr).unwrap();
        let settings = [("unit", D::Text("  ")), ("tabWidth", D::Number(4.0))];
        let raw = fields(&id, &c, action, D::Object(&settings));
        let q = normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        assert_eq!(q.argument_declaration_v1().variant.fields.len(), 10);
        let request = if action == "indent-lines" {
            LineEditV1::Indent {
                unit: "  ".into(),
                tab_width: 4,
            }
        } else {
            outdent()
        };
        native
            .submit_line_edit(&nid, &nc, &request, &mut nr)
            .unwrap();
        native.run(&nid, &mut nr).unwrap();
        o.submit_line_edit_arguments_v1(&q, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert_eq!(
            o.read(&c.source.document, &mut r).unwrap().source.text(),
            native
                .read(&nc.source.document, &mut nr)
                .unwrap()
                .source
                .text()
        );
        assert_eq!(
            o.logical_view(&c.view, &mut r).unwrap().selections,
            native.logical_view(&nc.view, &mut nr).unwrap().selections
        );
        let before = o.generation();
        assert_eq!(
            o.submit_line_edit_arguments_v1(&q, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), before);
        let changed = [("unit", D::Text("\t")), ("tabWidth", D::Number(4.0))];
        let raw = fields(&id, &c, action, D::Object(&changed));
        let changed =
            normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        assert_eq!(
            o.submit_line_edit_arguments_v1(&changed, &mut r),
            Err(InvocationError::IntentMismatch)
        );
    }
}
#[test]
fn explicit_newline_and_indent_modes_match_the_existing_native_job_with_capture_saturation() {
    for (name, delimiter) in [
        ("LF", GeneratedNewlineV1::Lf),
        ("CRLF", GeneratedNewlineV1::CrLf),
        ("CR", GeneratedNewlineV1::Cr),
    ] {
        for (indent_name, indent) in [
            ("none", NewlineIndentV1::None),
            ("copy-prefix", NewlineIndentV1::CopyPrefix),
        ] {
            for capacity in [0, 1, 64] {
                let (mut o, c, _) = fixture(4096);
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                let (mut native, nc, mut nr) = fixture(4096);
                let id = o.issue_invocation(&mut r).unwrap();
                let nid = native.issue_invocation(&mut nr).unwrap();
                let settings = [
                    ("delimiter", D::Text(name)),
                    ("autoIndent", D::Text(indent_name)),
                ];
                let raw = fields(&id, &c, "insert-newline", D::Object(&settings));
                let q = normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r)
                    .unwrap();
                assert_eq!(
                    q.owned_bytes(),
                    NormalizedLineEditArgumentsV1::required_owned_bytes(0).unwrap()
                );
                native
                    .submit_line_edit(
                        &nid,
                        &nc,
                        &LineEditV1::Newline { delimiter, indent },
                        &mut nr,
                    )
                    .unwrap();
                native.run(&nid, &mut nr).unwrap();
                o.submit_line_edit_arguments_v1(&q, &mut r).unwrap();
                o.run(&id, &mut r).unwrap();
                assert_eq!(
                    o.read(&c.source.document, &mut r).unwrap().source.text(),
                    native
                        .read(&nc.source.document, &mut nr)
                        .unwrap()
                        .source
                        .text()
                );
                assert_eq!(
                    o.logical_view(&c.view, &mut r).unwrap().selections,
                    native.logical_view(&nc.view, &mut nr).unwrap().selections
                );
            }
        }
    }
}
#[test]
fn line_envelope_and_settings_are_closed_not_ui_or_arbitrary_data() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let settings = [("unit", D::Text("  ")), ("tabWidth", D::Number(4.0))];
    let raw = fields(&id, &c, "indent-lines", D::Object(&settings));
    for i in 0..raw.len() {
        let mut bad = raw.clone();
        bad.remove(i);
        assert!(normalize_line_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
        let mut bad = raw.clone();
        bad[i] = raw[(i + 1) % raw.len()];
        assert!(normalize_line_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
    }
    for unit in ["", "x", " \t", "\u{a0}"] {
        let settings = [("unit", D::Text(unit)), ("tabWidth", D::Number(4.0))];
        let raw = fields(&id, &c, "indent-lines", D::Object(&settings));
        assert!(normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err());
    }
    for width in [
        0.0,
        -1.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        9_007_199_254_740_992.0,
    ] {
        let settings = [("unit", D::Text(" ")), ("tabWidth", D::Number(width))];
        let raw = fields(&id, &c, "indent-lines", D::Object(&settings));
        assert!(normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err());
    }
    let mut bad = raw.clone();
    bad.push(("keymap", V::Data(D::Null)));
    assert!(normalize_line_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn line_copy_limits_are_exact_and_newline_needs_no_unit_allocation() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let settings = [("unit", D::Text("  ")), ("tabWidth", D::Number(4.0))];
    let raw = fields(&id, &c, "indent-lines", D::Object(&settings));
    let needed = NormalizedLineEditArgumentsV1::required_owned_bytes(2).unwrap();
    for delta in [-1, 0, 1] {
        let l = LineEditArgumentLimitsV1 {
            max_unit_bytes: 2,
            max_owned_bytes: needed.checked_add_signed(delta).unwrap(),
        };
        assert_eq!(
            normalize_line_edit_arguments_v1(V::Object(&raw), &c, l, &mut r).is_ok(),
            delta >= 0
        );
    }
    assert!(
        normalize_line_edit_arguments_v1(
            V::Object(&raw),
            &c,
            LineEditArgumentLimitsV1 {
                max_unit_bytes: 1,
                ..limits()
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        NormalizedLineEditArgumentsV1::required_owned_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}

#[test]
fn unknown_settings_roles_or_defaults_never_fall_back_to_a_native_action() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let indent = [("unit", D::Text(" ")), ("tabWidth", D::Number(4.0))];
    for action in ["newline", "format-document", "", "copy-prefix"] {
        let raw = fields(&id, &c, action, D::Object(&indent));
        assert!(normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err());
    }
    for settings in [
        D::Null,
        D::Array(&[]),
        D::Object(&[("unit", D::Text(" "))]),
        D::Object(&[("unit", D::Text(" ")), ("unit", D::Text(" "))]),
        D::Object(&[
            ("unit", D::Text(" ")),
            ("tabWidth", D::Number(4.0)),
            ("keymap", D::Null),
        ]),
    ] {
        let raw = fields(&id, &c, "indent-lines", settings);
        assert!(normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err());
    }
    for (delimiter, auto) in [
        ("platform-default", "none"),
        ("LF", "grammar"),
        ("\n", "none"),
    ] {
        let settings = [
            ("delimiter", D::Text(delimiter)),
            ("autoIndent", D::Text(auto)),
        ];
        let raw = fields(&id, &c, "insert-newline", D::Object(&settings));
        assert!(normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err());
    }
    let raw = fields(&id, &c, "indent-lines", D::Object(&indent));
    for i in [0, 1, 3, 4, 5, 6, 7, 8] {
        let mut bad = raw.clone();
        bad[i].1 = V::Data(D::Text("forged"));
        assert!(normalize_line_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}

#[test]
fn syntactic_line_bounds_do_not_claim_native_work_budget_or_current_permission() {
    for service_loss in [false, true] {
        let (mut o, c, mut r) = fixture(4096);
        let id = o.issue_invocation(&mut r).unwrap();
        let settings = [
            ("unit", D::Text(" ")),
            (
                "tabWidth",
                D::Number(if service_loss { 4.0 } else { 129.0 }),
            ),
        ];
        let raw = fields(&id, &c, "indent-lines", D::Object(&settings));
        let q = normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        o.submit_line_edit_arguments_v1(&q, &mut r).unwrap();
        if service_loss {
            o.set_read_only(&c.source.document, true, &mut r).unwrap();
        }
        let before = o.generation();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::LineRejected(_)
        ));
        assert_eq!(o.generation(), before);
        assert_eq!(
            o.read(&c.source.document, &mut r).unwrap().source.text(),
            " \talpha\r\n"
        );
    }
}

#[test]
fn line_normalization_and_admission_timing_report_success_and_refusal_without_settings() {
    use vellum_engine::instrumentation::Disposition;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        let (mut o, c, _) = fixture(4096);
        let mut r = Recorder::try_with_capacity(64)
            .unwrap()
            .with_native_timing(timing);
        let id = o.issue_invocation(&mut r).unwrap();
        let settings = [("unit", D::Text("  ")), ("tabWidth", D::Number(4.0))];
        let raw = fields(&id, &c, "indent-lines", D::Object(&settings));
        let q = normalize_line_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        assert!(!format!("{q:?}").contains("tabWidth"));
        assert!(!format!("{:?}", V::Object(&raw)).contains("tabWidth"));
        o.submit_line_edit_arguments_v1(&q, &mut r).unwrap();
        let wrong = fields(&id, &c, "unknown", D::Object(&settings));
        assert!(normalize_line_edit_arguments_v1(V::Object(&wrong), &c, limits(), &mut r).is_err());
        let other = fields(&id, &c, "outdent-lines", D::Object(&settings));
        let other =
            normalize_line_edit_arguments_v1(V::Object(&other), &c, limits(), &mut r).unwrap();
        assert_eq!(
            o.submit_line_edit_arguments_v1(&other, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|v| {
                matches!(
                    v.boundary,
                    Boundary::LineArgumentNormalization | Boundary::LineDecodedAdmission
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
        assert!(!format!("{records:?}").contains("tabWidth"));
    }
}
