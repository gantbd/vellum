use super::*;
#[test]
fn annotation_frontier_receipts_keep_the_original_identity_through_absence_and_redo() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let (annotation, created) = create(&mut o, target(&source, 0, 1), "PRIVATE_BODY", &mut r);
    let group = created.history_group.unwrap();
    let c = SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "reversal-result/1".into(),
        continuity: "c".into(),
    };
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 4,
            max_work: 128,
        },
    };
    let original = HistoryReversalTargetV1::AnnotationFrontier {
        annotation: annotation.clone(),
        expected_group: group,
    };
    for direction in [HistoryDirection::Undo, HistoryDirection::Redo] {
        let id = o.issue_invocation(&mut r).unwrap();
        let grant = TrustedReversalResultGrantV1 {
            invocation: &id,
            authority: &c.authority,
            target: &original,
            disclose_target: true,
        };
        let f = TrustedReversalResultAdmissionV1 {
            context: &c,
            publication_epoch: 0,
            publication_allowed: Some(true),
            context_current: Some(true),
            result_admission_allowed: Some(true),
            max_result_bytes: ReversalResultV1::required_bytes(),
            target_grant: Some(grant),
        };
        o.submit_reversal_target_with_reads_and_result_v3(
            &id, &original, direction, &reads, &f, &mut r,
        )
        .unwrap();
        o.run(&id, &mut r).unwrap();
        let current = TrustedReversalResultReleaseV1 {
            context: &c,
            publication_epoch: 0,
            publication_allowed: Some(true),
            outcome_readable: Some(true),
            result_context_current: Some(true),
            reply_ready: Some(true),
            resources_ready: Some(true),
            max_result_bytes: ReversalResultV1::required_bytes(),
            target_grant: Some(grant),
        };
        let receipt = o
            .retained_reversal_result_v1(&id, &current, &mut r)
            .unwrap();
        assert!(
            matches!(receipt.decision, ReversalDecisionV1::Applied { publication, target: Some(ref returned) } if publication == id && *returned == original)
        );
        assert_eq!(
            o.annotation_usage().present_records,
            usize::from(direction == HistoryDirection::Redo)
        );
    }
}
