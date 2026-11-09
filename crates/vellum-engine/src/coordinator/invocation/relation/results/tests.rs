use super::super::tests::setup;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "native-relation-lifecycle/1".into(),
        continuity: "c".into(),
    }
}
fn admission(c: &SettingsExecutionContextV1) -> TrustedRelationResultAdmissionV1<'_> {
    TrustedRelationResultAdmissionV1 {
        context: c,
        max_result_bytes: RelationResultV1::required_bytes(),
        result_admission_allowed: Some(true),
        context_current: Some(true),
        relation_result_identifiers_allowed: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedRelationResultReleaseV1<'_> {
    TrustedRelationResultReleaseV1 {
        context: c,
        max_result_bytes: RelationResultV1::required_bytes(),
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
fn capture_mut<'a>(o: &'a mut SourceCoordinator, id: &InvocationId) -> &'a mut Capture {
    let Some(BoundIntent::Relation {
        result: Some(c), ..
    }) = &mut o.invocations.slots[id.slot].as_mut().unwrap().intent
    else {
        panic!("missing capture")
    };
    c
}
#[test]
fn exact_owned_capture_budget_is_required_before_binding_and_before_final_publication() {
    for workspace in [false, true] {
        for shrink in [false, true] {
            let (mut o, command, mut r) = setup(workspace);
            let c = context();
            let id = o.issue_invocation(&mut r).unwrap();
            let exact = TrustedRelationResultAdmissionV1::required_intent_bytes(&c).unwrap();
            o.invocations.limits.max_intent_bytes = exact - 1;
            assert_eq!(
                o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r),
                Err(InvocationError::Limit)
            );
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
            o.invocations.limits.max_intent_bytes = exact;
            o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r)
                .unwrap();
            let generation = o.generation();
            if shrink {
                o.invocations.limits.max_intent_bytes -= 1;
            }
            o.run(&id, &mut r).unwrap();
            if shrink {
                assert_eq!(o.generation(), generation);
                assert!(matches!(
                    o.invocation_outcome(&id, &mut r),
                    Ok(InvocationOutcome::RelationRejected(_))
                ));
                assert_eq!(
                    o.retained_relation_result_v1(&id, &release(&c), &mut r)
                        .err(),
                    Some(InvocationError::OutcomeUnavailable)
                );
                assert!(capture_mut(&mut o, &id).receipt.is_none());
            } else {
                assert!(
                    o.retained_relation_result_v1(&id, &release(&c), &mut r)
                        .is_ok()
                );
            }
        }
    }
}
#[test]
fn another_jobs_private_receipt_seal_cannot_be_delivered_for_the_original_ticket() {
    let (mut o, command, mut r) = setup(false);
    let c = context();
    let a = o.issue_invocation(&mut r).unwrap();
    let b = o.issue_invocation(&mut r).unwrap();
    for id in [&a, &b] {
        o.submit_relation_with_result_v1(id, &command, &admission(&c), &mut r)
            .unwrap();
        o.run(id, &mut r).unwrap();
    }
    let generation = o.generation();
    let stolen = capture_mut(&mut o, &b).receipt.clone();
    capture_mut(&mut o, &a).receipt = stolen;
    assert_eq!(
        o.retained_relation_result_v1(&a, &release(&c), &mut r)
            .err(),
        Some(InvocationError::Faulted)
    );
    assert!(
        o.retained_relation_result_v1(&b, &release(&c), &mut r)
            .is_ok()
    );
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
}
#[test]
fn changed_original_ticket_fences_before_root_or_identity_publication() {
    let (mut o, command, mut r) = setup(false);
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r)
        .unwrap();
    let other = o.issue_invocation(&mut r).unwrap();
    capture_mut(&mut o, &id).invocation = other;
    let generation = o.generation();
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.logical_view_usage().issued_lifetime, 0);
    assert!(capture_mut(&mut o, &id).receipt.is_none());
}
#[test]
fn interruption_after_root_publication_does_not_publish_a_success_receipt_or_known_rejection() {
    let (mut o, command, mut r) = setup(true);
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r)
        .unwrap();
    let generation = o.generation();
    o.relation_publication_fault = true;
    assert!(catch_unwind(AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    assert!(o.is_faulted());
    assert!(o.generation() > generation);
    assert_eq!(o.workspace_usage().issued_lifetime, 1);
    assert!(capture_mut(&mut o, &id).receipt.is_none());
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retained_relation_result_v1(&id, &release(&c), &mut r)
            .err(),
        Some(InvocationError::Unavailable)
    );
}
