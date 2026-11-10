use super::*;
use vellum_engine::coordinator::{
    HistoryReversalTargetV1, InvocationOutcome, InvocationState, ReadDependencyBudget,
    ReadDependencyRequestV3, ReadDependencyV1, ReadDependencyV2, ReadDependencyV3,
};
use vellum_engine::instrumentation::Boundary;

fn request(version: vellum_engine::coordinator::DocumentVersion) -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(version),
        ))],
        budget: ReadDependencyBudget {
            max_predicates: 4,
            max_work: 128,
        },
    }
}

#[test]
fn given_linked_history_when_reversed_with_declared_reads_then_all_members_publish_and_diagnostics_stay_optional()
 {
    for capacity in [0, 1, 128] {
        let mut o = invocation_owner();
        let mut r = Recorder::try_with_capacity(capacity).unwrap();
        let a = o.create(b"A", &mut r).unwrap().document;
        let b = o.create(b"B", &mut r).unwrap().document;
        let dependency = o.create(b"private dependency", &mut r).unwrap();
        let reads = request(dependency);
        let g = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r)
            .unwrap()
            .history_group
            .unwrap();
        for direction in [HistoryDirection::Undo, HistoryDirection::Redo] {
            while r.pop_front().is_some() {}
            let id = o.issue_invocation(&mut r).unwrap();
            let target = HistoryReversalTargetV1::SourceFrontier {
                document: a.clone(),
                expected_group: g.clone(),
            };
            o.submit_reversal_target_with_reads_v3(&id, &target, direction, &reads, &mut r)
                .unwrap();
            assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
            assert!(matches!(
                o.invocation_outcome(&id, &mut r),
                Ok(InvocationOutcome::Committed(_))
            ));
            if capacity == 128 {
                let mut boundaries = Vec::new();
                while let Some(observation) = r.pop_front() {
                    boundaries.push(observation.boundary);
                }
                for expected in [
                    Boundary::ReadDependencyPreparation,
                    Boundary::ReadDependencyValidation,
                    Boundary::HistoryPreparation,
                    Boundary::InvocationAdmission,
                    Boundary::InvocationExecution,
                    Boundary::Publication,
                ] {
                    assert!(boundaries.contains(&expected), "missing {expected:?}");
                }
            }
            let (expected_a, expected_b) = if direction == HistoryDirection::Undo {
                ("A", "B")
            } else {
                ("a", "b")
            };
            assert_eq!(o.read(&a, &mut r).unwrap().source.text(), expected_a);
            assert_eq!(o.read(&b, &mut r).unwrap().source.text(), expected_b);
        }
        assert!(r.queued() <= capacity);
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
    }
}

#[test]
fn given_pending_linked_reversal_when_a_declared_read_changes_then_no_member_is_published() {
    let mut o = invocation_owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let dependency = o.create(b"D", &mut r).unwrap();
    let reads = request(dependency.clone());
    let g = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &reads, &mut r)
        .unwrap();
    change(&mut o, &[(&dependency.document, "D", "d")], &mut r).unwrap();
    let generation = o.generation();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::Rejected(_))
    ));
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "a");
    assert_eq!(o.read(&b, &mut r).unwrap().source.text(), "b");
    assert!(!o.is_faulted());
}
