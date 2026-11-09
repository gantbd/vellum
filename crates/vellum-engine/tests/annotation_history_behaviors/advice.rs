use super::*;
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::selection::{Selection, SelectionBudget};
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "agent".into(),
        purpose: "validate".into(),
        destination: "native".into(),
        profile: "execution/1".into(),
        continuity: "session".into(),
    }
}
fn request<'a>(
    p: &'a CompoundIntent,
    c: &'a SettingsExecutionContextV1,
) -> CompoundAdviceRequestV1<'a> {
    CompoundAdviceRequestV1 {
        context: c,
        policy: "approved",
        profile: NATIVE_COMPOUND_ADVICE_PROFILE_V1,
        plan: p,
        reads: None,
        scopes: None,
        limits: CompoundAdviceLimitsV1 {
            max_request_bytes: 32768,
            max_label_bytes: 128,
            max_reply_bytes: 128,
        },
    }
}
fn facts<'a>(
    q: &'a CompoundAdviceRequestV1<'a>,
    b: &'a CompoundAdviceBasisV1<'a>,
) -> TrustedCompoundAdviceFactsV1<'a> {
    TrustedCompoundAdviceFactsV1 {
        request: q,
        basis: b,
        context: q.context,
        policy: q.policy,
        domain_bound: Some(true),
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        proposal_readable: Some(true),
        validation_observable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
    }
}
fn setup() -> (SourceCoordinator, Recorder, DocumentVersion, CompoundIntent) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    let d = o.create(b"body", &mut r).unwrap();
    let p = CompoundIntent {
        source: SourceIntent {
            writes: vec![write(d.clone(), 0, 4, "body", "new")],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: target(&d, 0, 4),
            body: "note".into(),
        }],
    };
    (o, r, d, p)
}
#[test]
fn given_mixed_proposal_and_attached_view_when_advice_passes_then_no_live_state_or_identity_is_spent()
 {
    for mode in 0..5 {
        let (mut o, mut r, d, p) = setup();
        o.configure_logical_views(
            LogicalViewLimits {
                max_live_views: 4,
                max_issued_views: 4,
                max_total_selections: 8,
                normalization: SelectionBudget {
                    max_input_selections: 8,
                    max_output_selections: 8,
                    max_work: 4096,
                },
                max_mapping_steps: 4096,
            },
            &mut r,
        )
        .unwrap();
        let view = o.create_logical_view(&mut r).unwrap();
        let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        o.attach_logical_view(
            &view,
            stamp,
            &d,
            &[Selection {
                anchor: BytePosition(4),
                head: BytePosition(4),
                affinity: Affinity::After,
            }],
            0,
            &mut r,
        )
        .unwrap();
        let snapshot = o.logical_view(&view, &mut r).unwrap();
        let c = context();
        let binding =
            (mode >= 3).then(|| crate::inspection_read::Reader::install_advice(&mut o, &c, &mut r));
        let q = request(&p, &c);
        let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
        let before = (
            o.generation(),
            o.annotation_usage(),
            o.history_usage(),
            o.invocation_usage(),
            o.retention_usage(),
            o.semantic_root_usage(&mut r).unwrap(),
        );
        assert_eq!(
            if mode == 0 {
                o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r)
            } else {
                envelope(&q, mode == 1 || mode == 3, |e| match &binding {
                    Some(binding) => binding.advice(&mut o, e, &facts(&q, &b), &mut r),
                    None => o.query_decoded_compound_advice_v1(e, &facts(&q, &b), &mut r),
                })
            },
            CompoundAdviceReplyV1::Advice {
                profile: NATIVE_COMPOUND_ADVICE_PROFILE_V1,
                checks_satisfied: true
            }
        );
        assert_eq!(
            (
                o.generation(),
                o.annotation_usage(),
                o.history_usage(),
                o.invocation_usage(),
                o.retention_usage(),
                o.semantic_root_usage(&mut r).unwrap()
            ),
            before
        );
        let current = o.logical_view(&view, &mut r).unwrap();
        assert_eq!(current.stamp, snapshot.stamp);
        assert_eq!(current.source, snapshot.source);
        assert_eq!(
            current.selections.selections(),
            snapshot.selections.selections()
        );
        if let Some(binding) = binding {
            binding.retire(&mut o, &mut r);
        }
        let prepared = o
            .prepare_compound(&p.source.writes, &[], &p.annotations, &mut r)
            .unwrap();
        let result = o.commit(prepared, &mut r).unwrap();
        assert_eq!(result.annotation_changes.len(), 1);
        assert_eq!(o.annotation_usage().issued_lifetime, 1);
        assert_eq!(o.history_usage().retained_groups, 1);
        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "new");
    }
}
fn envelope<'h, T>(
    q: &'h CompoundAdviceRequestV1<'h>,
    typed: bool,
    consume: impl for<'a> FnOnce(&'a CompoundAdviceEnvelopeValueV1<'a, 'h>) -> T,
) -> T {
    use CompoundAdviceEnvelopeValueV1 as V;
    use vellum_engine::capture::InertValueV1 as D;
    let args = [
        ("contract", V::Data(D::Text("proposal-validation/1"))),
        ("operation", V::Data(D::Text("advisory-validation"))),
        ("plan", V::Plan(q.plan)),
    ];
    let header = [
        (
            if typed { "family" } else { "name" },
            V::Data(if typed {
                D::Number(9.0)
            } else {
                D::Text("validate-proposal")
            }),
        ),
        ("variant", V::Data(D::Text("advisory-validation"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
        ("proposal", V::Proposal(q)),
    ];
    consume(&V::Object(&outer))
}
#[test]
fn given_positive_advice_when_policy_changes_before_service_then_normal_publication_still_refuses()
{
    let (mut o, mut r, d, p) = setup();
    let c = context();
    let q = request(&p, &c);
    let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    assert!(matches!(
        o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
        CompoundAdviceReplyV1::Advice {
            checks_satisfied: true,
            ..
        }
    ));
    assert_eq!(o.invocation_usage().issued_lifetime, 0);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_compound(&id, &p, &mut r).unwrap();
    o.set_read_only(&d.document, true, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(_)
    ));
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "body");
    assert_eq!(o.annotation_usage().present_records, 0);
    assert_eq!(o.history_usage().retained_groups, 0);
}
