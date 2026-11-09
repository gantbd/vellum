use super::*;

#[test]
fn given_an_annotation_frontier_when_reversed_with_a_source_read_then_creation_and_restoration_keep_the_original_owner()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"prose", &mut r).unwrap();
    let (annotation, outcome) = create(&mut o, target(&source, 0, 5), "comment", &mut r);
    let group = outcome.history_group.unwrap();
    let target = HistoryReversalTargetV1::AnnotationFrontier {
        annotation: annotation.clone(),
        expected_group: group,
    };
    let request = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(source.clone()),
        ))],
        budget: ReadDependencyBudget {
            max_predicates: 1,
            max_work: 16,
        },
    };
    for direction in [HistoryDirection::Undo, HistoryDirection::Redo] {
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_target_with_reads_v3(&id, &target, direction, &request, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::Committed(_))
        ));
        assert_eq!(
            o.read(&source.document, &mut r).unwrap().source.text(),
            "prose"
        );
        if direction == HistoryDirection::Undo {
            assert!(o.annotation(&annotation, &mut r).is_err());
        } else {
            let restored = o.annotation(&annotation, &mut r).unwrap();
            assert_eq!(restored.body, "comment");
            assert_eq!(restored.authored.source.document, source.document);
        }
    }
}
