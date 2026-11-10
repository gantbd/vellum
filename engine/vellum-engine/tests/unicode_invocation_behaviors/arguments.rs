use super::*;
use BoundaryEditEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::instrumentation::NativeTimingProfileV1;
fn limits() -> BoundaryEditArgumentLimitsV1 {
    BoundaryEditArgumentLimitsV1 {
        max_owned_bytes: 4096,
    }
}
fn fields<'a>(
    id: &'a InvocationId,
    c: &'a LogicalViewContext,
    evidence: &'a BoundaryEvidenceId,
    unit: &'a str,
    direction: &'a str,
    count: f64,
) -> Vec<(&'a str, V<'a, 'a>)> {
    vec![
        ("contract", V::Data(D::Text("boundary-command/1"))),
        ("operation", V::Data(D::Text("ordinary-edit"))),
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
        ("profile", V::Data(D::Text("unicode-logical-edit/1"))),
        ("boundary", V::Boundary(evidence)),
        ("unit", V::Data(D::Text(unit))),
        ("direction", V::Data(D::Text(direction))),
        ("count", V::Data(D::Number(count))),
        ("extend", V::Data(D::Boolean(false))),
    ]
}
#[test]
fn boundary_envelope_deletions_match_native_unit_direction_and_count_semantics() {
    for (unit_name, unit) in [
        ("grapheme", UnicodeUnitV1::Grapheme),
        ("word-start", UnicodeUnitV1::WordStart),
        ("word-end", UnicodeUnitV1::WordEnd),
    ] {
        for (dir_name, direction) in [
            ("forward", UnicodeDirectionV1::Forward),
            ("backward", UnicodeDirectionV1::Backward),
        ] {
            for count in [1, 2] {
                let (mut o, c, e, mut r) = fixture(4096);
                let (mut native, nc, ne, mut nr) = fixture(4096);
                let id = o.issue_invocation(&mut r).unwrap();
                let nid = native.issue_invocation(&mut nr).unwrap();
                let raw = fields(&id, &c, &e, unit_name, dir_name, count as f64);
                let q = normalize_boundary_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r)
                    .unwrap();
                assert_eq!(q.argument_declaration_v1().variant.fields.len(), 13);
                native
                    .submit_unicode_command(
                        &nid,
                        &nc,
                        &ne,
                        UnicodeOperationV1::Deletion,
                        UnicodeCommandV1 {
                            unit,
                            direction,
                            count,
                            extend: false,
                        },
                        &mut nr,
                    )
                    .unwrap();
                native.run(&nid, &mut nr).unwrap();
                o.submit_boundary_edit_arguments_v1(&q, &mut r).unwrap();
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
                o.retire_boundary_evidence(&e, &mut r).unwrap();
                let before = o.generation();
                assert_eq!(
                    o.submit_boundary_edit_arguments_v1(&q, &mut r),
                    Ok(InvocationState::Finished)
                );
                assert_eq!(o.generation(), before);
            }
        }
    }
}
#[test]
fn boundary_envelope_is_closed_and_does_not_accept_data_as_evidence_or_ui_units() {
    let (mut o, c, e, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = fields(&id, &c, &e, "grapheme", "forward", 1.0);
    for i in 0..raw.len() {
        let mut bad = raw.clone();
        bad.remove(i);
        assert!(
            normalize_boundary_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
        let mut bad = raw.clone();
        bad[i] = raw[(i + 1) % raw.len()];
        assert!(
            normalize_boundary_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
        let mut bad = raw.clone();
        bad[i].1 = V::Data(D::Text("forged"));
        assert!(
            normalize_boundary_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
    }
    for count in [
        0.0,
        -1.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        9_007_199_254_740_992.0,
    ] {
        let raw = fields(&id, &c, &e, "grapheme", "forward", count);
        assert!(
            normalize_boundary_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err()
        );
    }
    let mut bad = raw.clone();
    bad[12].1 = V::Data(D::Boolean(true));
    assert!(normalize_boundary_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
    let mut bad = raw.clone();
    bad.push(("keymap", V::Data(D::Null)));
    assert!(normalize_boundary_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err());
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn fixed_boundary_output_has_exact_owned_header_bounds_without_a_table_copy() {
    let (mut o, c, e, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = fields(&id, &c, &e, "grapheme", "forward", 1.0);
    let needed = NormalizedBoundaryEditArgumentsV1::required_owned_bytes();
    let before = o.boundary_usage();
    for delta in [-1, 0, 1] {
        let l = BoundaryEditArgumentLimitsV1 {
            max_owned_bytes: needed.checked_add_signed(delta).unwrap(),
        };
        assert_eq!(
            normalize_boundary_edit_arguments_v1(V::Object(&raw), &c, l, &mut r).is_ok(),
            delta >= 0
        );
    }
    let q = normalize_boundary_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    assert_eq!(q.owned_bytes(), needed);
    assert_eq!(q.evidence(), &e);
    assert_eq!(o.boundary_usage(), before);
}

#[test]
fn boundary_representation_does_not_grant_live_evidence_native_work_or_write_permission() {
    for case in 0..4 {
        let (mut o, c, e, mut r) = fixture(4096);
        let (_, _, foreign, _) = fixture(4096);
        let id = o.issue_invocation(&mut r).unwrap();
        let evidence = if case == 0 { &foreign } else { &e };
        if case == 1 {
            o.retire_boundary_evidence(&e, &mut r).unwrap();
        }
        let raw = fields(
            &id,
            &c,
            evidence,
            "grapheme",
            "forward",
            if case == 2 { 65.0 } else { 1.0 },
        );
        let q =
            normalize_boundary_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        o.submit_boundary_edit_arguments_v1(&q, &mut r).unwrap();
        if case == 3 {
            o.set_read_only(&c.source.document, true, &mut r).unwrap();
        }
        let before = o.generation();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::UnicodeRejected(_)
        ));
        assert_eq!(o.generation(), before);
        assert_eq!(
            o.read(&c.source.document, &mut r).unwrap().source.text(),
            "a\u{301} hi!"
        );
    }
}

#[test]
fn boundary_actual_normalization_and_admission_timing_are_optional_and_payload_free() {
    use vellum_engine::instrumentation::Disposition;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 128] {
            let (mut o, c, e, _) = fixture(4096);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let id = o.issue_invocation(&mut r).unwrap();
            let raw = fields(&id, &c, &e, "word-start", "forward", 1.0);
            let q = normalize_boundary_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r)
                .unwrap();
            assert!(!format!("{q:?}").contains("word-start"));
            assert!(!format!("{:?}", V::Object(&raw)).contains("word-start"));
            o.submit_boundary_edit_arguments_v1(&q, &mut r).unwrap();
            let bad = fields(&id, &c, &e, "word-start", "forward", 0.0);
            assert!(
                normalize_boundary_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r)
                    .is_err()
            );
            let changed = fields(&id, &c, &e, "word-start", "forward", 2.0);
            let changed =
                normalize_boundary_edit_arguments_v1(V::Object(&changed), &c, limits(), &mut r)
                    .unwrap();
            assert_eq!(
                o.submit_boundary_edit_arguments_v1(&changed, &mut r),
                Err(InvocationError::IntentMismatch)
            );
            o.run(&id, &mut r).unwrap();
            assert_eq!(
                o.read(&c.source.document, &mut r).unwrap().source.text(),
                "hi!"
            );
            if capacity == 128 {
                let records: Vec<_> = std::iter::from_fn(|| r.pop_front())
                    .filter(|v| {
                        matches!(
                            v.boundary,
                            Boundary::BoundaryEditArgumentNormalization
                                | Boundary::BoundaryEditDecodedAdmission
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
                assert!(records.iter().all(|v| v.service_time.is_some()
                    == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                assert!(!format!("{records:?}").contains("word-start"));
            } else if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
}
