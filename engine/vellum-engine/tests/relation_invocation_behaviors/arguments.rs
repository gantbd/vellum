use super::*;
use RelationEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::instrumentation::NativeTimingProfileV1;

fn rows<'a, 'h: 'a>(
    id: &'h InvocationId,
    operation: &'a str,
    action: &'a str,
) -> Vec<(&'a str, V<'a, 'h>)> {
    vec![
        ("contract", V::Data(D::Text("relation-invocation/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("native-relation-lifecycle/1"))),
        ("action", V::Data(D::Text(action))),
    ]
}
fn limits() -> RelationArgumentLimitsV1 {
    RelationArgumentLimitsV1 {
        max_owned_bytes: NormalizedRelationArgumentsV1::required_owned_bytes(),
    }
}

#[test]
fn four_native_container_envelopes_delegate_to_existing_one_shot_jobs_without_implicit_targets() {
    for workspace in [false, true] {
        let (mut o, mut r) = owner();
        let operation = if workspace {
            "workspace-container"
        } else {
            "view-container"
        };
        let id = o.issue_invocation(&mut r).unwrap();
        let input = rows(&id, operation, "create");
        let before = o.generation();
        let issued = o.invocation_usage().issued_lifetime;
        let args = normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).unwrap();
        assert!(std::ptr::eq(args.invocation(), &id));
        assert_eq!(args.argument_declaration_v1().variant.fields.len(), 5);
        assert_eq!(o.generation(), before);
        assert_eq!(o.invocation_usage().issued_lifetime, issued);
        assert_eq!(
            o.submit_relation_arguments_v1(&args, &mut r),
            Ok(InvocationState::Pending)
        );
        o.run(&id, &mut r).unwrap();
        let InvocationOutcome::RelationCommitted(receipt) =
            o.invocation_outcome(&id, &mut r).unwrap()
        else {
            panic!("missing creation")
        };
        let receipt = receipt.clone();
        let created = o.generation();
        assert_eq!(
            o.submit_relation_arguments_v1(&args, &mut r),
            Ok(InvocationState::Finished)
        );
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), created);
        let retire_id = o.issue_invocation(&mut r).unwrap();
        let mut input = rows(&retire_id, operation, "retire");
        let stamp;
        let context;
        match &receipt {
            RelationReceiptV1::ViewCreated(view) => {
                stamp = o.logical_view(view, &mut r).unwrap().stamp;
                input.extend([
                    ("target", V::View(view)),
                    ("expected", V::ViewStamp(&stamp)),
                ]);
            }
            RelationReceiptV1::WorkspaceCreated(workspace) => {
                context = o.workspace(workspace, &mut r).unwrap().context;
                input.extend([
                    ("target", V::Workspace(workspace)),
                    ("expected", V::WorkspaceContext(&context)),
                ]);
            }
            _ => panic!("wrong creation"),
        }
        let args = normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).unwrap();
        assert_eq!(args.argument_declaration_v1().variant.fields.len(), 7);
        o.submit_relation_arguments_v1(&args, &mut r).unwrap();
        o.run(&retire_id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&retire_id, &mut r),
            Ok(InvocationOutcome::RelationCommitted(
                RelationReceiptV1::ViewRetired(_) | RelationReceiptV1::WorkspaceRetired(_)
            ))
        ));
        let retired = o.generation();
        o.forget_invocation_outcome(&retire_id, &mut r).unwrap();
        assert_eq!(
            o.submit_relation_arguments_v1(&args, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), retired);
    }
}

#[test]
fn exact_fields_closed_literals_and_native_roles_refuse_without_binding_or_publication() {
    let (mut o, mut r) = owner();
    let id = o.issue_invocation(&mut r).unwrap();
    let before = o.generation();
    for index in 0..5 {
        let mut input = rows(&id, "view-container", "create");
        input.remove(index);
        assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
        let mut input = rows(&id, "view-container", "create");
        input.push(input[index]);
        assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
        let mut input = rows(&id, "view-container", "create");
        input[index].1 = V::Data(D::Null);
        assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
    }
    for (index, text) in [
        (0, "unknown/1"),
        (1, "document-container"),
        (3, "unknown/1"),
        (4, "toggle"),
    ] {
        let mut input = rows(&id, "view-container", "create");
        input[index].1 = V::Data(D::Text(text));
        assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
    }
    let mut input = rows(&id, "view-container", "create");
    input.push(("target", V::Data(D::Text("assigned"))));
    assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.generation(), before);
}

#[test]
fn exact_fixed_owned_budget_and_optional_timing_do_not_change_native_meaning() {
    for capacity in [0, 1, 32] {
        for timed in [false, true] {
            let (mut o, _) = owner();
            let mut setup = Recorder::disabled();
            let id = o.issue_invocation(&mut setup).unwrap();
            let input = rows(&id, "workspace-container", "create");
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(if timed {
                    NativeTimingProfileV1::ProcessMonotonicV1
                } else {
                    NativeTimingProfileV1::Off
                });
            let args =
                normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).unwrap();
            assert_eq!(args.owned_bytes(), limits().max_owned_bytes);
            assert!(!format!("{args:?}").contains("workspace-container"));
            let short = RelationArgumentLimitsV1 {
                max_owned_bytes: limits().max_owned_bytes - 1,
            };
            assert_eq!(
                normalize_relation_arguments_v1(V::Object(&input), short, &mut r).err(),
                Some(InvocationError::Limit)
            );
            o.submit_relation_arguments_v1(&args, &mut r).unwrap();
            o.run(&id, &mut r).unwrap();
            assert_eq!(o.workspace_usage().issued_lifetime, 1);
            assert_eq!(o.workspace_usage().live, 1);
            let mut seen = 0;
            while let Some(record) = r.pop_front() {
                if matches!(
                    record.boundary,
                    Boundary::RelationArgumentNormalization | Boundary::RelationDecodedAdmission
                ) {
                    assert_eq!(record.service_time.is_some(), timed);
                    seen += 1;
                }
            }
            if capacity > 1 {
                assert_eq!(seen, 3);
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
}

#[test]
fn retirement_roles_require_exact_target_agreement_without_looking_up_or_rebinding_names() {
    let (mut o, mut r) = owner();
    let a = o.create_workspace(&mut r).unwrap();
    let b = o.create_workspace(&mut r).unwrap();
    let view = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
    let id = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    let mut input = rows(&id, "workspace-container", "retire");
    input.extend([
        ("target", V::Workspace(&a.workspace)),
        ("expected", V::WorkspaceContext(&b)),
    ]);
    assert_eq!(
        normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).err(),
        Some(InvocationError::IntentMismatch)
    );
    for (operation, target, expected) in [
        (
            "view-container",
            V::Workspace(&a.workspace),
            V::ViewStamp(&stamp),
        ),
        (
            "workspace-container",
            V::View(&view),
            V::WorkspaceContext(&a),
        ),
        ("view-container", V::View(&view), V::WorkspaceContext(&a)),
        (
            "workspace-container",
            V::Workspace(&a.workspace),
            V::ViewStamp(&stamp),
        ),
        (
            "view-container",
            V::Data(D::Text("view")),
            V::ViewStamp(&stamp),
        ),
    ] {
        let mut input = rows(&id, operation, "retire");
        input.extend([("target", target), ("expected", expected)]);
        assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
    }
    let mut input = rows(&id, "view-container", "retire");
    input.extend([
        ("target", V::View(&view)),
        ("expected", V::ViewStamp(&stamp)),
    ]);
    input[6].0 = "target";
    assert!(normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).is_err());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}

#[test]
fn foreign_or_stale_native_expectations_are_preserved_for_native_refusal_not_refreshed() {
    for foreign in [false, true] {
        let (mut o, mut r) = owner();
        let (mut other, _) = owner();
        let view = if foreign {
            other.create_logical_view(&mut r).unwrap()
        } else {
            o.create_logical_view(&mut r).unwrap()
        };
        let expected = LogicalViewStamp {
            attachment_generation: u64::MAX,
            selection_generation: u64::MAX,
            viewport_generation: u64::MAX,
        };
        let id = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        let mut input = rows(&id, "view-container", "retire");
        input.extend([
            ("target", V::View(&view)),
            ("expected", V::ViewStamp(&expected)),
        ]);
        let args = normalize_relation_arguments_v1(V::Object(&input), limits(), &mut r).unwrap();
        assert_eq!(
            args.request(),
            &RelationCommandV1::RetireView {
                view: view.clone(),
                expected
            }
        );
        o.submit_relation_arguments_v1(&args, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::RelationRejected(_))
        ));
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
    }
}
