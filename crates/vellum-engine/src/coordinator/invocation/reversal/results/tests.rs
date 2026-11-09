use super::super::tests::fixture;
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "reversal-result/1".into(),
        continuity: "c".into(),
    }
}
fn admission(c: &SettingsExecutionContextV1) -> TrustedReversalResultAdmissionV1<'_> {
    TrustedReversalResultAdmissionV1 {
        context: c,
        publication_epoch: 0,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: ReversalResultV1::required_bytes(),
        target_grant: None,
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedReversalResultReleaseV1<'_> {
    TrustedReversalResultReleaseV1 {
        context: c,
        publication_epoch: 0,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: ReversalResultV1::required_bytes(),
        target_grant: None,
    }
}
fn result_state_mut<'a>(o: &'a mut SourceCoordinator, id: &InvocationId) -> &'a mut State {
    state_mut(&mut o.invocations.slots[id.slot].as_mut().unwrap().intent).unwrap()
}
#[test]
fn complete_private_receipt_and_maximum_frame_are_funded_before_direct_or_read_admission() {
    for dependent in [false, true] {
        let (mut o, _, group, mut r) = fixture();
        let c = context();
        let id = o.issue_invocation(&mut r).unwrap();
        let target = HistoryReversalTargetV1::Group(group.clone());
        let reads = ReadDependencyRequestV3 {
            predicates: vec![],
            budget: ReadDependencyBudget {
                max_predicates: 4,
                max_work: 128,
            },
        };
        let exact = if dependent {
            TrustedReversalResultAdmissionV1::required_read_intent_bytes(&c, &reads).unwrap()
        } else {
            TrustedReversalResultAdmissionV1::required_direct_intent_bytes(&c).unwrap()
        };
        o.invocations.limits.max_intent_bytes = exact - 1;
        let submit = |o: &mut SourceCoordinator, r: &mut Recorder| {
            if dependent {
                o.submit_reversal_target_with_reads_and_result_v3(
                    &id,
                    &target,
                    HistoryDirection::Undo,
                    &reads,
                    &admission(&c),
                    r,
                )
            } else {
                o.submit_reversal_with_result_v1(
                    &id,
                    &group,
                    HistoryDirection::Undo,
                    &admission(&c),
                    r,
                )
            }
        };
        assert_eq!(submit(&mut o, &mut r), Err(InvocationError::Limit));
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        o.invocations.limits.max_intent_bytes = exact;
        submit(&mut o, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.retained_reversal_result_v1(&id, &release(&c), &mut r)
                .unwrap()
                .decision,
            ReversalDecisionV1::Applied { .. }
        ));
    }
}
#[test]
fn final_resource_refusal_keeps_a_confirmed_minimal_nonpublication_decision() {
    let (mut o, document, group, mut r) = fixture();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &admission(&c), &mut r)
        .unwrap();
    o.invocations.limits.max_intent_bytes =
        TrustedReversalResultAdmissionV1::required_direct_intent_bytes(&c).unwrap() - 1;
    let generation = o.generation();
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.retained_reversal_result_v1(&id, &release(&c), &mut r)
            .unwrap()
            .decision,
        ReversalDecisionV1::NotPublished
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.documents[document.slot].as_ref().unwrap().source.text(),
        "a"
    );
    assert!(!o.is_faulted());
}
#[test]
fn substituted_original_capture_cannot_publish_or_fabricate_a_known_refusal() {
    let (mut o, document, group, mut r) = fixture();
    let c = context();
    let a = o.issue_invocation(&mut r).unwrap();
    let b = o.issue_invocation(&mut r).unwrap();
    for id in [&a, &b] {
        o.submit_reversal_with_result_v1(
            id,
            &group,
            HistoryDirection::Undo,
            &admission(&c),
            &mut r,
        )
        .unwrap();
    }
    let stolen = result_state_mut(&mut o, &b).original.clone();
    result_state_mut(&mut o, &a).original = stolen;
    let generation = o.generation();
    assert_eq!(o.run(&a, &mut r), Err(InvocationError::Faulted));
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.documents[document.slot].as_ref().unwrap().source.text(),
        "a"
    );
    assert!(result_state_mut(&mut o, &a).receipt.is_none());
}
#[test]
fn private_causal_anchor_substitution_withholds_only_the_corrupt_receipt() {
    let (mut o, _, group, mut r) = fixture();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &admission(&c), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let other = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    result_state_mut(&mut o, &id)
        .receipt
        .as_mut()
        .unwrap()
        .decision = Decision::Applied(other);
    assert_eq!(
        o.retained_reversal_result_v1(&id, &release(&c), &mut r)
            .err(),
        Some(InvocationError::Faulted)
    );
    assert!(!o.is_faulted());
    assert_eq!(o.generation(), generation);
}
#[test]
fn prepublication_frontier_fault_and_midinstallation_unwind_never_install_a_receipt() {
    for partial in [false, true] {
        let (mut o, document, group, mut r) = fixture();
        let c = context();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_with_result_v1(
            &id,
            &group,
            HistoryDirection::Undo,
            &admission(&c),
            &mut r,
        )
        .unwrap();
        if partial {
            o.publication_fault_after = Some(1);
            assert!(catch_unwind(AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
        } else {
            o.reversal_stage_fault = Some(history::reversal_validation::Fault::WrongState);
            assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        }
        assert!(o.is_faulted());
        assert!(result_state_mut(&mut o, &id).receipt.is_none());
        assert_eq!(
            o.documents[document.slot].as_ref().unwrap().source.text(),
            if partial { "A" } else { "a" }
        );
        assert_eq!(
            o.retained_reversal_result_v1(&id, &release(&c), &mut r)
                .err(),
            Some(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
    }
}
