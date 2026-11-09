use super::*;
use crate::coordinator::proposal_advice::tests::{facts, owner_with_parent, plan, request};
pub(in crate::coordinator) fn packet<'h, T>(
    q: &'h CompoundAdviceRequestV1<'h>,
    plan: &'h CompoundIntent,
    typed: bool,
    case: usize,
    consume: impl for<'v> FnOnce(&'v V<'v, 'h>) -> T,
) -> T {
    let mut args = [
        ("contract", V::Data(D::Text("proposal-validation/1"))),
        ("operation", V::Data(D::Text("advisory-validation"))),
        ("plan", V::Plan(plan)),
        ("extra", V::Data(D::Null)),
    ];
    match case {
        8 => args[0].1 = V::Data(D::Text("proposal-validation/2")),
        9 => args[1].1 = V::Data(D::Text("ordinary-edit")),
        11 => args[2].0 = "contract",
        12 => args[2].1 = V::Proposal(q),
        15 => args[0].1 = V::Data(D::Object(&[])),
        _ => {}
    }
    let mut header = [
        (
            if typed { "family" } else { "name" },
            V::Data(if typed {
                D::Number(if case == 4 { 10.0 } else { 9.0 })
            } else {
                D::Text(if case == 4 {
                    "read-document"
                } else {
                    "validate-proposal"
                })
            }),
        ),
        (
            "variant",
            V::Data(D::Text(if case == 5 {
                "surprise"
            } else {
                "advisory-validation"
            })),
        ),
        (
            "version",
            V::Data(D::Number(if case == 6 { 2.0 } else { 1.0 })),
        ),
        (
            "arguments",
            V::Object(&args[..if case == 10 { 4 } else { 3 }]),
        ),
        ("extra", V::Data(D::Null)),
    ];
    if case == 16 {
        header[1].0 = "version";
    }
    let mut outer = [
        (
            "route",
            V::Data(D::Text(if case == 3 {
                "surprise"
            } else if typed {
                "typed"
            } else {
                "dynamic"
            })),
        ),
        (
            "request",
            V::Object(&header[..if case == 7 { 5 } else { 4 }]),
        ),
        (
            "proposal",
            if case == 13 {
                V::Data(D::Null)
            } else {
                V::Proposal(q)
            },
        ),
        ("extra", V::Data(D::Null)),
    ];
    if case == 2 {
        outer[2].0 = "route";
    }
    consume(&V::Object(&outer[..if case == 1 { 4 } else { 3 }]))
}
#[test]
fn closed_advice_headers_refuse_before_native_work_and_preserve_original_plan_and_proposal_roles() {
    for typed in [false, true] {
        let (mut o, mut r, d) = owner_with_parent(1048576);
        let p = plan(&d, 1);
        let copy = p.clone();
        let context = crate::coordinator::serialization::tests::context();
        let q = request(&p, &context);
        let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
        let f = facts(&q, &b);
        let before = (
            o.generation(),
            o.annotation_usage(),
            o.history_usage(),
            o.invocation_usage(),
            o.retention_usage(),
        );
        for case in 1..17 {
            r = Recorder::try_with_capacity(128).unwrap();
            let original = if case == 14 { &copy } else { &p };
            let out = packet(&q, original, typed, case, |e| {
                o.query_decoded_compound_advice_v1(e, &f, &mut r)
            });
            assert_eq!(out, CompoundAdviceReplyV1::Unavailable, "case={case}");
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(!events.iter().any(|e| matches!(
                e.boundary,
                Boundary::CompoundAdviceAdmission
                    | Boundary::CompoundAdviceDecision
                    | Boundary::CompoundAdviceRelease
            )));
            assert!(!o.is_faulted());
        }
        assert_eq!(
            (
                o.generation(),
                o.annotation_usage(),
                o.history_usage(),
                o.invocation_usage(),
                o.retention_usage()
            ),
            before
        );
        let out = packet(&q, &p, typed, 0, |e| {
            o.query_decoded_compound_advice_v1(e, &f, &mut r)
        });
        assert!(matches!(
            out,
            CompoundAdviceReplyV1::Advice {
                checks_satisfied: true,
                ..
            }
        ));
    }
}
