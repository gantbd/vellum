use super::*;
use SelectionEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
#[path = "../support/selection_arguments.rs"]
mod support;
#[test]
fn boundary_selection_envelope_preserves_original_evidence_and_routes_movement_not_deletion() {
    for unit in ["grapheme", "word-start", "word-end"] {
        for extend in [false, true] {
            for installed in [false, true] {
                let (mut o, c, evidence, mut r) = fixture(16384);
                ordinary_result::fund(&mut o, &mut r);
                let binding = installed.then(|| selection_route::Route::install(&mut o, &mut r));
                o.set_read_only(&c.source.document, true, &mut r).unwrap();
                let id = o.issue_invocation(&mut r).unwrap();
                ordinary_result::bind(&mut o, &id, &mut r);
                let mut raw = support::header(&id, &c, "logical-movement");
                raw[0].1 = V::Data(D::Text("boundary-command/1"));
                raw[7].1 = V::Data(D::Text("unicode-logical-edit/1"));
                raw.extend([
                    ("boundary", V::Boundary(&evidence)),
                    ("unit", V::Data(D::Text(unit))),
                    ("direction", V::Data(D::Text("forward"))),
                    ("count", V::Data(D::Number(1.0))),
                    ("extend", V::Data(D::Boolean(extend))),
                ]);
                let q = normalize_selection_arguments_v1(
                    V::Object(&raw),
                    &c,
                    support::limits(),
                    &mut r,
                )
                .unwrap();
                let (original, command) = q.boundary_request().unwrap();
                assert!(std::ptr::eq(original, &evidence));
                assert_eq!(command.extend, extend);
                assert_eq!(q.argument_declaration_v1().variant.fields.len(), 13);
                assert_eq!(
                    selection_route::submit(binding.as_ref(), &mut o, &q, &mut r),
                    Ok(InvocationState::Pending)
                );
                o.run(&id, &mut r).unwrap();
                assert!(matches!(
                    o.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::UnicodeMovementCommitted(_)
                ));
                assert_eq!(
                    o.read(&c.source.document, &mut r).unwrap().version,
                    c.source
                );
                assert_eq!(o.history_usage().retained_groups, 0);
                assert_eq!(
                    selection_route::submit(binding.as_ref(), &mut o, &q, &mut r),
                    Ok(InvocationState::Finished)
                );
                raw[11].1 = V::Data(D::Number(0.0));
                ordinary_result::check(&mut o, &id, InvocationPublicationV1::Applied, &mut r);
                if let Some(binding) = binding {
                    binding.retire(&mut o, &mut r);
                }
                assert_eq!(
                    normalize_selection_arguments_v1(
                        V::Object(&raw),
                        &c,
                        support::limits(),
                        &mut r
                    )
                    .unwrap_err(),
                    InvocationError::MalformedIntent
                );
                raw[11].1 = V::Data(D::Number(1.0));
                raw[8].1 = V::Data(D::Null);
                assert_eq!(
                    normalize_selection_arguments_v1(
                        V::Object(&raw),
                        &c,
                        support::limits(),
                        &mut r
                    )
                    .unwrap_err(),
                    InvocationError::MalformedIntent
                );
            }
        }
    }
}
