use super::*;
use LiteralEditEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::instrumentation::NativeTimingProfileV1;

fn limits() -> LiteralEditArgumentLimitsV1 {
    LiteralEditArgumentLimitsV1 {
        max_text_bytes: 1024,
        max_owned_bytes: 4096,
    }
}
fn fields<'a>(
    id: &'a InvocationId,
    c: &'a LogicalViewContext,
    text: &'a str,
    after: &'a str,
) -> Vec<(&'a str, V<'a, 'a>)> {
    vec![
        ("contract", V::Data(D::Text("selection-edit/1"))),
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
        (
            "profile",
            V::Data(D::Text("explicit-positional-selection/1")),
        ),
        ("text", V::Data(D::Text(text))),
        ("selectionAfter", V::Data(D::Text(after))),
    ]
}
#[test]
fn decoded_literal_edits_use_the_original_native_ledger_and_replay_without_reacquiring_context() {
    for after in ["mapped", "carets-after"] {
        for capacity in [0, 1, 64] {
            let (mut o, c, _) = fixture(4096);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let id = o.issue_invocation(&mut r).unwrap();
            let payload = "Deutsch: Grüße\r\n```mermaid\ngraph LR\n```";
            let raw = fields(&id, &c, payload, after);
            let before = o.generation();
            let q =
                normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
            assert_eq!(o.generation(), before);
            assert_eq!(q.argument_declaration_v1().variant.fields.len(), 10);
            assert_eq!(q.text(), payload);
            assert!(!format!("{q:?}").contains("Deutsch"));
            assert!(!format!("{:?}", V::Object(&raw)).contains("Deutsch"));
            assert_eq!(
                o.submit_literal_edit_arguments_v1(&q, &mut r),
                Ok(InvocationState::Pending)
            );
            assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
            let generation = o.generation();
            assert_eq!(generation, before + 1);
            assert_eq!(
                o.read(&c.source.document, &mut r).unwrap().source.text(),
                payload
            );
            let expected = if after == "mapped" {
                selection(0, payload.len())
            } else {
                selection(payload.len(), payload.len())
            };
            assert_eq!(
                o.logical_view(&c.view, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[expected]
            );
            assert_eq!(
                o.submit_literal_edit_arguments_v1(&q, &mut r),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
            let changed = fields(&id, &c, "other", after);
            let changed =
                normalize_literal_edit_arguments_v1(V::Object(&changed), &c, limits(), &mut r)
                    .unwrap();
            assert_eq!(
                o.submit_literal_edit_arguments_v1(&changed, &mut r),
                Err(InvocationError::IntentMismatch)
            );
        }
    }
}
#[test]
fn literal_header_is_closed_and_opaque_roles_cannot_be_replaced_by_data_or_other_handles() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = fields(&id, &c, "x", "mapped");
    for index in 0..raw.len() {
        let mut missing = raw.clone();
        missing.remove(index);
        assert!(
            normalize_literal_edit_arguments_v1(V::Object(&missing), &c, limits(), &mut r).is_err()
        );
        let mut duplicate = raw.clone();
        duplicate[index] = raw[(index + 1) % raw.len()];
        assert!(
            normalize_literal_edit_arguments_v1(V::Object(&duplicate), &c, limits(), &mut r)
                .is_err()
        );
        let mut data = raw.clone();
        data[index].1 = V::Data(D::Text("forged"));
        if raw[index].0 != "text" {
            assert!(
                normalize_literal_edit_arguments_v1(V::Object(&data), &c, limits(), &mut r)
                    .is_err()
            );
        }
    }
    let mut extra = raw.clone();
    extra.push(("keymap", V::Data(D::Null)));
    assert!(normalize_literal_edit_arguments_v1(V::Object(&extra), &c, limits(), &mut r).is_err());
    let (_, other, _) = fixture(4096);
    let mut foreign = raw.clone();
    foreign[3].1 = V::View(&other.view);
    assert!(
        normalize_literal_edit_arguments_v1(V::Object(&foreign), &c, limits(), &mut r).is_err()
    );
    foreign = raw.clone();
    foreign[4].1 = V::Document(&other.source.document);
    assert!(
        normalize_literal_edit_arguments_v1(V::Object(&foreign), &c, limits(), &mut r).is_err()
    );
    for number in [
        -1.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        9_007_199_254_740_992.0,
        c.source.revision as f64 + 1.0,
    ] {
        let mut bad = raw.clone();
        bad[5].1 = V::Data(D::Number(number));
        assert!(
            normalize_literal_edit_arguments_v1(V::Object(&bad), &c, limits(), &mut r).is_err()
        );
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn literal_flat_copy_has_exact_text_and_owned_bounds_without_a_schema_tree_copy() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = fields(&id, &c, "Grüße", "mapped");
    let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    let required = NormalizedLiteralEditArgumentsV1::required_owned_bytes("Grüße".len()).unwrap();
    assert_eq!(q.owned_bytes(), required);
    for delta in [-1, 0, 1] {
        let l = LiteralEditArgumentLimitsV1 {
            max_text_bytes: "Grüße".len(),
            max_owned_bytes: required.checked_add_signed(delta).unwrap(),
        };
        assert_eq!(
            normalize_literal_edit_arguments_v1(V::Object(&raw), &c, l, &mut r).is_ok(),
            delta >= 0
        );
    }
    assert!(
        normalize_literal_edit_arguments_v1(
            V::Object(&raw),
            &c,
            LiteralEditArgumentLimitsV1 {
                max_text_bytes: "Grüße".len() - 1,
                ..limits()
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        NormalizedLiteralEditArgumentsV1::required_owned_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}
#[test]
fn normalized_literal_roles_do_not_grant_authority_or_override_service_read_only_changes() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = fields(&id, &c, "x", "carets-after");
    let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    o.submit_literal_edit_arguments_v1(&q, &mut r).unwrap();
    o.set_read_only(&c.source.document, true, &mut r).unwrap();
    let before = o.generation();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(_)
    ));
    assert_eq!(o.generation(), before);
    assert_eq!(
        o.read(&c.source.document, &mut r).unwrap().source.text(),
        "abc"
    );
    let (mut other, _, _) = fixture(4096);
    let foreign = other.issue_invocation(&mut r).unwrap();
    let raw = fields(&foreign, &c, "x", "mapped");
    let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    assert_eq!(
        o.submit_literal_edit_arguments_v1(&q, &mut r),
        Err(InvocationError::Unavailable)
    );
}

#[test]
fn equal_literal_bytes_keep_mapped_noop_distinct_from_caret_only_publication() {
    for after in ["mapped", "carets-after"] {
        let (mut o, c, mut r) = fixture(4096);
        let id = o.issue_invocation(&mut r).unwrap();
        let raw = fields(&id, &c, "abc", after);
        let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        let before = o.generation();
        o.submit_literal_edit_arguments_v1(&q, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), before + u64::from(after == "carets-after"));
        assert_eq!(
            o.read(&c.source.document, &mut r).unwrap().version,
            c.source
        );
        let expected = if after == "mapped" {
            selection(0, 3)
        } else {
            selection(3, 3)
        };
        assert_eq!(
            o.logical_view(&c.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[expected]
        );
    }
}

#[test]
fn literal_normalization_and_decoded_admission_report_actual_service_not_payloads() {
    use vellum_engine::instrumentation::Disposition;
    for profile in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        let (mut o, c, _) = fixture(4096);
        let mut r = Recorder::try_with_capacity(64)
            .unwrap()
            .with_native_timing(profile);
        let id = o.issue_invocation(&mut r).unwrap();
        let mut raw = fields(&id, &c, "private literal", "mapped");
        let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        o.submit_literal_edit_arguments_v1(&q, &mut r).unwrap();
        raw.pop();
        assert!(
            normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).is_err()
        );
        let other = fields(&id, &c, "changed", "mapped");
        let q =
            normalize_literal_edit_arguments_v1(V::Object(&other), &c, limits(), &mut r).unwrap();
        assert_eq!(
            o.submit_literal_edit_arguments_v1(&q, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|v| {
                matches!(
                    v.boundary,
                    Boundary::LiteralArgumentNormalization | Boundary::LiteralDecodedAdmission
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
                == (profile == NativeTimingProfileV1::ProcessMonotonicV1))
        );
        assert!(!format!("{records:?}").contains("private literal"));
    }
}

#[test]
fn empty_literal_and_reordered_header_are_lossless_not_a_missing_argument() {
    for after in ["mapped", "carets-after"] {
        let (mut o, c, mut r) = fixture(4096);
        let id = o.issue_invocation(&mut r).unwrap();
        let mut raw = fields(&id, &c, "", after);
        raw.reverse();
        let l = LiteralEditArgumentLimitsV1 {
            max_text_bytes: 0,
            max_owned_bytes: NormalizedLiteralEditArgumentsV1::required_owned_bytes(0).unwrap(),
        };
        let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, l, &mut r).unwrap();
        assert_eq!(q.text(), "");
        o.submit_literal_edit_arguments_v1(&q, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert_eq!(
            o.read(&c.source.document, &mut r).unwrap().source.text(),
            ""
        );
    }
}

#[test]
fn syntactically_valid_old_context_is_not_rebuilt_from_newer_live_selections() {
    let (mut o, c, mut r) = fixture(4096);
    let id = o.issue_invocation(&mut r).unwrap();
    o.set_logical_selections(&c, &[selection(1, 1)], 0, &mut r)
        .unwrap();
    let before = o.generation();
    let raw = fields(&id, &c, "x", "mapped");
    let q = normalize_literal_edit_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    assert_eq!(q.context(), &c);
    assert_eq!(
        o.submit_literal_edit_arguments_v1(&q, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(_)
    ));
    assert_eq!(o.generation(), before);
    assert_eq!(
        o.read(&c.source.document, &mut r).unwrap().source.text(),
        "abc"
    );
    assert_eq!(
        o.logical_view(&c.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[selection(1, 1)]
    );
}
