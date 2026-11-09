use super::*;
use SelectionEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
#[path = "../support/selection_arguments.rs"]
mod support;
#[test]
fn search_selection_envelope_uses_owned_occurrence_and_all_closed_selection_modes() {
    for mode in ["replace-all", "replace-primary", "add-primary"] {
        for installed in [false, true] {
            let (mut o, c, _, result, mut r) = fixture(128, &[s(0, 1, Affinity::After)]);
            ordinary_result::fund(&mut o, &mut r);
            let binding = installed.then(|| selection_route::Route::install(&mut o, &mut r));
            let occurrence = result.match_id(1).unwrap();
            let id = o.issue_invocation(&mut r).unwrap();
            ordinary_result::bind(&mut o, &id, &mut r);
            let mut raw = support::header(&id, &c, "select-search-match");
            raw.extend([
                ("match", V::Match(&occurrence)),
                ("direction", V::Data(D::Text("backward"))),
                ("affinity", V::Data(D::Text("before"))),
                ("selectionMode", V::Data(D::Text(mode))),
            ]);
            let q =
                normalize_selection_arguments_v1(V::Object(&raw), &c, support::limits(), &mut r)
                    .unwrap();
            let SelectionCommandV1::SearchMatch {
                occurrence: original,
                ..
            } = q.selection_request().unwrap()
            else {
                panic!("wrong native variant")
            };
            assert_eq!(original, &occurrence);
            assert_eq!(q.argument_declaration_v1().variant.fields.len(), 12);
            let mut exact = support::limits();
            exact.max_owned_bytes = q.owned_bytes();
            assert!(normalize_selection_arguments_v1(V::Object(&raw), &c, exact, &mut r).is_ok());
            exact.max_owned_bytes -= 1;
            assert_eq!(
                normalize_selection_arguments_v1(V::Object(&raw), &c, exact, &mut r).unwrap_err(),
                InvocationError::Limit
            );
            selection_route::submit(binding.as_ref(), &mut o, &q, &mut r).unwrap();
            o.run(&id, &mut r).unwrap();
            let view = o.logical_view(&c.view, &mut r).unwrap();
            let selected = s(5, 3, Affinity::Before);
            let expected = if mode == "add-primary" {
                vec![s(0, 1, Affinity::After), selected]
            } else {
                vec![selected]
            };
            assert_eq!(view.selections.selections(), expected);
            assert_eq!(o.history_usage().retained_groups, 0);
            assert_eq!(
                o.read(&c.source.document, &mut r).unwrap().version,
                c.source
            );
            assert_eq!(
                selection_route::submit(binding.as_ref(), &mut o, &q, &mut r),
                Ok(InvocationState::Finished)
            );
            raw[8].1 = V::Data(D::Text("3..5 is not occurrence evidence"));
            ordinary_result::check(&mut o, &id, InvocationPublicationV1::Applied, &mut r);
            if let Some(binding) = binding {
                binding.retire(&mut o, &mut r);
            }
            assert_eq!(
                normalize_selection_arguments_v1(V::Object(&raw), &c, support::limits(), &mut r)
                    .unwrap_err(),
                InvocationError::MalformedIntent
            );
        }
    }
}
