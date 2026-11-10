use super::*;
use SelectionEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
#[path = "../support/selection_arguments.rs"]
mod support;
use support::{header, limits};
fn endpoint_fields<'h>(
    id: &'h InvocationId,
    c: &'h LogicalViewContext,
) -> Vec<(&'h str, V<'h, 'h>)> {
    let mut raw = header(id, c, "logical-movement");
    raw.extend([
        ("unit", V::Data(D::Text("document-end"))),
        ("extend", V::Data(D::Boolean(false))),
    ]);
    raw
}
#[test]
fn set_and_endpoint_envelopes_reach_the_original_one_shot_view_only_handler() {
    for endpoint in [false, true] {
        for installed in [false, true] {
            for capture in 0..4 {
                let (mut o, c, mut r) = fixture();
                ordinary_result::fund(&mut o, &mut r);
                let binding = installed.then(|| selection_route::Route::install(&mut o, &mut r));
                r = if capture == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if capture == 3 { 1 } else { 256 }).unwrap()
                };
                if capture >= 2 {
                    r = r.with_native_timing(
                        vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                    );
                }
                o.set_read_only(&c.source.document, true, &mut r).unwrap();
                let id = o.issue_invocation(&mut r).unwrap();
                ordinary_result::bind(&mut o, &id, &mut r);
                let fields = [
                    ("anchor", D::Number(2.0)),
                    ("head", D::Number(0.0)),
                    ("affinity", D::Text("before")),
                ];
                let selections = [D::Object(&fields)];
                let mut raw = header(
                    &id,
                    &c,
                    if endpoint {
                        "logical-movement"
                    } else {
                        "set-selection"
                    },
                );
                if endpoint {
                    raw.extend([
                        ("unit", V::Data(D::Text("line-end"))),
                        ("extend", V::Data(D::Boolean(true))),
                    ]);
                } else {
                    raw.extend([
                        ("selections", V::Data(D::Array(&selections))),
                        ("primary", V::Data(D::Number(0.0))),
                    ]);
                }
                let generation = o.generation();
                let history = o.history_usage();
                let q = normalize_selection_arguments_v1(V::Object(&raw), &c, limits(), &mut r)
                    .unwrap();
                assert!(std::ptr::eq(q.invocation(), &id));
                assert!(std::ptr::eq(q.context(), &c));
                assert_eq!(q.argument_declaration_v1().variant.fields.len(), 10);
                assert_eq!(o.generation(), generation);
                assert_eq!(
                    selection_route::submit(binding.as_ref(), &mut o, &q, &mut r),
                    Ok(InvocationState::Pending)
                );
                assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
                let result = o.logical_view(&c.view, &mut r).unwrap();
                let expected = if endpoint {
                    Selection {
                        anchor: BytePosition(1),
                        head: BytePosition(3),
                        affinity: Affinity::After,
                    }
                } else {
                    Selection {
                        anchor: BytePosition(2),
                        head: BytePosition(0),
                        affinity: Affinity::Before,
                    }
                };
                assert_eq!(result.selections.selections(), &[expected]);
                assert_eq!(
                    o.read(&c.source.document, &mut r).unwrap().version,
                    c.source
                );
                assert_eq!(o.history_usage(), history);
                let generation = o.generation();
                assert_eq!(
                    selection_route::submit(binding.as_ref(), &mut o, &q, &mut r),
                    Ok(InvocationState::Finished)
                );
                ordinary_result::check(&mut o, &id, InvocationPublicationV1::Applied, &mut r);
                assert_eq!(o.generation(), generation);
                if let Some(binding) = binding {
                    binding.retire(&mut o, &mut r);
                }
                let observations: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                if capture == 1 || capture == 2 {
                    let dispatches: Vec<_> = observations
                        .iter()
                        .filter(|e| e.boundary == Boundary::DiscoverySelectionDispatch)
                        .collect();
                    assert_eq!(dispatches.len(), if installed { 2 } else { 0 });
                    assert!(
                        dispatches
                            .iter()
                            .all(|e| e.service_time.is_some() == (capture == 2))
                    );
                } else if capture == 3 {
                    assert!(r.dropped() > 0);
                } else {
                    assert!(observations.is_empty());
                }
            }
        }
    }
}

#[test]
fn selection_headers_are_closed_and_never_coerce_roles_numbers_or_visual_commands() {
    let (mut o, c, mut r) = fixture();
    let id = o.issue_invocation(&mut r).unwrap();
    let before = o.generation();
    for case in 0..14 {
        let mut raw = endpoint_fields(&id, &c);
        match case {
            0 => {
                raw.pop();
            }
            1 => {
                raw[9] = raw[8];
            }
            2 => raw[0].1 = V::Data(D::Text("selection-edit/2")),
            3 => raw[1].1 = V::Data(D::Text("ordinary-edit")),
            4 => raw[2].1 = V::Data(D::Text("not-an-invocation")),
            5 => raw[3].1 = V::Document(&c.source.document),
            6 => raw[4].1 = V::View(&c.view),
            7 => raw[5].1 = V::Data(D::Number(f64::NAN)),
            8 => raw[5].1 = V::Data(D::Number(9_007_199_254_740_992.0)),
            9 => raw[6].1 = V::Data(D::Number(c.stamp.selection_generation as f64 + 1.0)),
            10 => raw[7].1 = V::Data(D::Text("unicode-logical-edit/1")),
            11 => raw[8].1 = V::Data(D::Text("visual-line")),
            12 => raw[9].1 = V::Data(D::Number(1.0)),
            _ => raw[9].0 = "extra",
        }
        assert_eq!(
            normalize_selection_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap_err(),
            InvocationError::MalformedIntent
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        assert_eq!(o.generation(), before);
    }
}

#[test]
fn raw_selection_order_primary_and_independent_count_and_byte_bounds_survive_normalization() {
    let (mut o, c, mut r) = fixture();
    let id = o.issue_invocation(&mut r).unwrap();
    let fields = [
        ("anchor", D::Number(2.0)),
        ("head", D::Number(0.0)),
        ("affinity", D::Text("before")),
    ];
    let values = [D::Object(&fields), D::Object(&fields)];
    let mut raw = header(&id, &c, "set-selection");
    raw.extend([
        ("selections", V::Data(D::Array(&values))),
        ("primary", V::Data(D::Number(1.0))),
    ]);
    let q = normalize_selection_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    let SelectionCommandV1::Set {
        selections,
        primary,
    } = q.selection_request().unwrap()
    else {
        panic!("wrong native variant")
    };
    assert_eq!(selections.len(), 2);
    assert_eq!(*primary, 1);
    assert_eq!(selections[0], selections[1]);
    assert_eq!(selections[0].anchor, BytePosition(2));
    let mut exact = limits();
    exact.max_owned_bytes = q.owned_bytes();
    assert!(normalize_selection_arguments_v1(V::Object(&raw), &c, exact, &mut r).is_ok());
    exact.max_owned_bytes -= 1;
    assert_eq!(
        normalize_selection_arguments_v1(V::Object(&raw), &c, exact, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    let mut count = limits();
    count.max_input_selections = 1;
    assert_eq!(
        normalize_selection_arguments_v1(V::Object(&raw), &c, count, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert!(!format!("{q:?}").contains("anchor"));
}

#[test]
fn nested_selection_rows_have_exact_fields_and_safe_noncoercing_coordinates() {
    let (mut o, c, mut r) = fixture();
    let id = o.issue_invocation(&mut r).unwrap();
    for case in 0..8 {
        let mut fields = [
            ("anchor", D::Number(0.0)),
            ("head", D::Number(2.0)),
            ("affinity", D::Text("after")),
        ];
        match case {
            0 => fields[0].1 = D::Number(-1.0),
            1 => fields[0].1 = D::Number(0.5),
            2 => fields[1].1 = D::Number(f64::INFINITY),
            3 => fields[1].1 = D::Number(9_007_199_254_740_992.0),
            4 => fields[1].1 = D::Text("2"),
            5 => fields[1].0 = "anchor",
            6 => fields[2].1 = D::Text("left"),
            _ => fields[2].0 = "extra",
        }
        let values = [D::Object(&fields)];
        let mut raw = header(&id, &c, "set-selection");
        raw.extend([
            ("selections", V::Data(D::Array(&values))),
            ("primary", V::Data(D::Number(0.0))),
        ]);
        assert_eq!(
            normalize_selection_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap_err(),
            InvocationError::MalformedIntent
        );
    }
}

#[test]
fn endpoint_bounds_are_original_intent_and_cannot_share_a_ticket_with_an_edit() {
    let (mut o, c, mut r) = fixture();
    let id = o.issue_invocation(&mut r).unwrap();
    let raw = endpoint_fields(&id, &c);
    let q = normalize_selection_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
    o.submit_selection_arguments_v1(&q, &mut r).unwrap();
    let mut changed = limits();
    changed.endpoint.max_scan_bytes -= 1;
    let other = normalize_selection_arguments_v1(V::Object(&raw), &c, changed, &mut r).unwrap();
    assert_eq!(
        o.submit_selection_arguments_v1(&other, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.submit_literal_edit(&id, &c, "not a keymap", SelectionAfterV1::Mapped, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(
        o.submit_selection_arguments_v1(&q, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
}

#[test]
fn argument_and_native_handoff_observation_is_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1, Observation};
    for mode in 0..4 {
        let (mut o, c, mut setup) = fixture();
        let id = o.issue_invocation(&mut setup).unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let raw = endpoint_fields(&id, &c);
        let q = normalize_selection_arguments_v1(V::Object(&raw), &c, limits(), &mut r).unwrap();
        assert_eq!(
            o.submit_selection_arguments_v1(&q, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
        let mut seen = [0; 2];
        while let Some(event) = r.pop_front() {
            let index = match event.boundary {
                Boundary::SelectionArgumentNormalization => Some(0),
                Boundary::SelectionDecodedAdmission => Some(1),
                _ => None,
            };
            if let Some(index) = index {
                seen[index] += 1;
                assert_eq!(event.disposition, Disposition::Completed);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
        }
        assert_eq!(seen, [usize::from(mode == 1 || mode == 2); 2]);
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
