//! Private-protocol regressions through the actual compound publisher.
use super::*;
fn fixture() -> (SourceCoordinator, HistoryGroupId, Recorder) {
    let mut o = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 4,
                max_edit_payload_bytes: 1024,
            },
        },
        HistoryLimits {
            max_retained_groups: 8,
            max_journal_bytes: 65536,
            max_pins: 4,
        },
    );
    let mut r = Recorder::disabled();
    let mut writes = Vec::new();
    for text in ["A", "B"] {
        let d = o.create(text.as_bytes(), &mut r).unwrap();
        writes.push(DocumentEdits {
            expected: d,
            edits: vec![Replacement {
                start: 0,
                end: 1,
                expected: text.into(),
                inserted: text.to_lowercase(),
            }],
        });
    }
    let p = o.prepare(&writes, &[], &mut r).unwrap();
    let group = o.commit(p, &mut r).unwrap().history_group.unwrap();
    (o, group, r)
}
#[test]
fn incomplete_or_retargeted_owned_reversal_cannot_publish_any_linked_participant() {
    for change in 0..2 {
        let (mut o, group, mut r) = fixture();
        let mut prepared = o
            .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap();
        if change == 0 {
            prepared.candidates.pop();
        } else {
            prepared.candidates[0].edits[0].inserted = "Z".into();
            prepared.candidates[0].source =
                Utf8Source::decode(b"Z", o.limits.source.max_source_bytes).unwrap();
        }
        let before = o.generation();
        let versions: Vec<_> = o
            .documents
            .iter()
            .flatten()
            .map(|d| (d.revision, d.history.version))
            .collect();
        assert_eq!(
            o.commit(prepared, &mut r).err(),
            Some(CoordinatorError::History(HistoryError::Faulted))
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), before);
        assert_eq!(
            o.documents
                .iter()
                .flatten()
                .map(|d| d.source.text())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(
            o.documents
                .iter()
                .flatten()
                .map(|d| (d.revision, d.history.version))
                .collect::<Vec<_>>(),
            versions
        );
    }
}

#[test]
fn malformed_staged_reversal_frontiers_fault_before_any_root_installation() {
    use reversal_validation::Fault::*;
    for fault in [
        MissingScope,
        DuplicateScope,
        WrongState,
        WrongGroup,
        WrongVersion,
        MissingOpposite,
        UnchangedFrom,
        LostFlag,
        Invalidated,
        UnexpectedGroup,
        MissingOutcome,
    ] {
        let (mut o, group, _) = fixture();
        let mut r = Recorder::try_with_capacity(64)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        let prepared = o
            .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap();
        let generation = o.generation();
        let bytes = o.history.bytes;
        o.reversal_stage_fault = Some(fault);
        assert_eq!(
            o.commit(prepared, &mut r).err(),
            Some(CoordinatorError::History(HistoryError::Faulted)),
            "{fault:?}"
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.history.bytes, bytes);
        assert_eq!(o.group(&group).unwrap().state, GroupState::Applied);
        let mut observed = false;
        while let Some(record) = r.pop_front() {
            if record.boundary == Boundary::HistoryReversalCandidateValidation {
                assert_eq!(record.disposition, Disposition::Unknown);
                assert!(record.service_time.is_some());
                observed = true;
            }
        }
        assert!(observed);
        assert_eq!(
            o.documents
                .iter()
                .flatten()
                .map(|d| d.source.text())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}

#[test]
fn exact_linked_undo_redo_accepts_permuted_members_and_optional_saturated_observation() {
    use crate::instrumentation::NativeTimingProfileV1;
    for capacity in [0, 1, 64] {
        for timed in [false, true] {
            let (mut o, group, _) = fixture();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(if timed {
                    NativeTimingProfileV1::ProcessMonotonicV1
                } else {
                    NativeTimingProfileV1::Off
                });
            for direction in [HistoryDirection::Undo, HistoryDirection::Redo] {
                let mut p = o.prepare_reversal(&group, direction, &mut r).unwrap();
                p.candidates.reverse();
                let PreparedHistory::Reversal { stamps, .. } = &mut p.history else {
                    unreachable!()
                };
                stamps.reverse();
                o.commit(p, &mut r).unwrap();
                let expected = if direction == HistoryDirection::Undo {
                    ["A", "B"]
                } else {
                    ["a", "b"]
                };
                assert_eq!(
                    o.documents
                        .iter()
                        .flatten()
                        .map(|d| d.source.text())
                        .collect::<Vec<_>>(),
                    expected
                );
            }
            let mut seen = 0;
            while let Some(record) = r.pop_front() {
                if record.boundary == Boundary::HistoryReversalCandidateValidation {
                    assert_eq!(record.disposition, Disposition::Completed);
                    assert_eq!(record.service_time.is_some(), timed);
                    seen += 1;
                }
            }
            if capacity == 64 {
                assert_eq!(seen, 2);
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
            assert!(!o.is_faulted());
        }
    }
}
